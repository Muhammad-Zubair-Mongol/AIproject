use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::{Duration, interval, timeout, Instant, sleep};
use crossbeam_channel::Receiver;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

// ============================================================================
// GEMINI CLIENT - With Rate Limiting & Smart Batching
// ============================================================================

const GEMINI_REST_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

// RATE LIMITING CONFIG
const MIN_REQUEST_INTERVAL_SECS: u64 = 3;      // Minimum 3 seconds between requests
const INITIAL_BACKOFF_SECS: u64 = 5;           // Start with 5 second backoff
const MAX_BACKOFF_SECS: u64 = 60;              // Max 60 second backoff
const RATE_LIMIT_CODES: [&str; 3] = ["429", "RESOURCE_EXHAUSTED", "rate"];

// AUDIO BATCHING CONFIG
const MIN_SPEECH_SECS: f32 = 3.0;              // Minimum 3 seconds of speech
const SILENCE_TIMEOUT_SECS: f32 = 2.0;         // 2 seconds silence = end
const MAX_BATCH_SECS: f32 = 15.0;              // Max 15 seconds per batch
const SPEECH_THRESHOLD: f32 = 0.001;           // 0.001 to resolve "waiting for speech" (was 0.02)
const SILENCE_THRESHOLD: f32 = 0.0005;         // Even lower for silence


pub struct GeminiState {
    pub audio_rx: StdMutex<Option<Receiver<Vec<f32>>>>,
    pub api_key: StdMutex<Option<String>>,
    pub is_connected: StdMutex<bool>,
    pub selected_model: StdMutex<String>,
}

impl Default for GeminiState {
    fn default() -> Self {
        Self {
            audio_rx: StdMutex::new(None),
            api_key: StdMutex::new(None),
            is_connected: StdMutex::new(false),
            selected_model: StdMutex::new("gemini-2.5-flash-preview-09-2025".to_string()),
        }
    }
}

const GOD_PROMPT_V9: &str = r#"You are a PASSIVE MEETING INTELLIGENCE ENGINE.

OUTPUT FORMAT - JSON ONLY:
{"transcript":"exact text","speaker":"Speaker 1","tone":"NEUTRAL","category":["INFO"],"confidence":0.85}

RULES:
- JSON only, no markdown
- Transcribe accurately (English/Urdu/Hindi)
- tone: NEUTRAL|URGENT|FRUSTRATED|EXCITED|POSITIVE|NEGATIVE
- category: TASK|DECISION|DEADLINE|QUERY|ACTION_ITEM|RISK|INFO
- If silence/unclear: {"status":"silence"}"#;

// ============================================================================
// Structs
// ============================================================================

#[derive(Serialize)]
struct RestRequest {
    contents: Vec<Content>,
    system_instruction: Option<SystemInstruction>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content { parts: Vec<Part> }

#[derive(Serialize)]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineData>,
}

#[derive(Serialize)]
struct InlineData { mime_type: String, data: String }

#[derive(Serialize)]
struct SystemInstruction { parts: Vec<TextPart> }

#[derive(Serialize)]
struct TextPart { text: String }

#[derive(Serialize)]
struct GenerationConfig { temperature: f32, max_output_tokens: i32 }

#[derive(Deserialize, Debug)]
struct RestResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
struct Candidate { content: Option<CandidateContent> }

#[derive(Deserialize, Debug)]
struct CandidateContent { parts: Option<Vec<ResponsePart>> }

#[derive(Deserialize, Debug)]
struct ResponsePart { text: Option<String> }

#[derive(Deserialize, Debug)]
struct ApiError { message: Option<String>, code: Option<i32> }

// ============================================================================
// Audio Helpers
// ============================================================================

fn to_wav(samples: &[f32]) -> Vec<u8> {
    let n = samples.len();
    let data_size = (n * 2) as u32;
    let mut wav = Vec::with_capacity(44 + n * 2);
    
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&16000u32.to_le_bytes());
    wav.extend_from_slice(&32000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    
    for s in samples {
        wav.extend_from_slice(&((*s * 32767.0).clamp(-32768.0, 32767.0) as i16).to_le_bytes());
    }
    wav
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

// ============================================================================
// API Call with Rate Limiting
// ============================================================================

async fn call_gemini_with_backoff(
    key: &str,
    model: &str,
    audio: &[f32],
    backoff: &mut u64,
    last_request: &mut Instant,
) -> Result<String, String> {
    // Enforce minimum interval
    let elapsed = last_request.elapsed();
    let min_interval = Duration::from_secs(MIN_REQUEST_INTERVAL_SECS);
    if elapsed < min_interval {
        let wait = min_interval - elapsed;
        println!("[GEMINI] Rate limit: waiting {:.1}s", wait.as_secs_f32());
        sleep(wait).await;
    }
    
    // Apply backoff if we had errors
    if *backoff > 0 {
        println!("[GEMINI] Backoff: waiting {}s", backoff);
        sleep(Duration::from_secs(*backoff)).await;
    }
    
    *last_request = Instant::now();
    
    let wav = to_wav(audio);
    let b64 = BASE64.encode(&wav);
    
    let request = RestRequest {
        contents: vec![Content {
            parts: vec![
                Part { text: Some("Analyze this audio:".into()), inline_data: None },
                Part { text: None, inline_data: Some(InlineData { 
                    mime_type: "audio/wav".into(), 
                    data: b64 
                })},
            ],
        }],
        system_instruction: Some(SystemInstruction {
            parts: vec![TextPart { text: GOD_PROMPT_V9.into() }],
        }),
        generation_config: GenerationConfig { temperature: 0.1, max_output_tokens: 512 },
    };
    
    let url = format!("{}/{}:generateContent?key={}", GEMINI_REST_URL, model, key);
    
    let client = reqwest::Client::new();
    let response = client.post(&url)
        .json(&request)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    
    let status = response.status();
    let text = response.text().await.map_err(|e| format!("Read: {}", e))?;
    
    // Check for rate limiting
    let is_rate_limited = status.as_u16() == 429 
        || RATE_LIMIT_CODES.iter().any(|code| text.contains(code));
    
    if is_rate_limited {
        // Exponential backoff
        *backoff = (*backoff * 2).max(INITIAL_BACKOFF_SECS).min(MAX_BACKOFF_SECS);
        println!("[GEMINI] ⚠️ Rate limited! Backoff now: {}s", backoff);
        return Err(format!("Rate limited. Waiting {}s before retry.", backoff));
    }
    
    // Success - reset backoff
    *backoff = 0;
    
    // Parse response
    if let Ok(resp) = serde_json::from_str::<RestResponse>(&text) {
        if let Some(error) = resp.error {
            return Err(format!("API: {}", error.message.unwrap_or_default()));
        }
        if let Some(c) = resp.candidates.and_then(|c| c.into_iter().next()) {
            if let Some(content) = c.content {
                if let Some(parts) = content.parts {
                    if let Some(part) = parts.into_iter().next() {
                        if let Some(t) = part.text {
                            return Ok(t);
                        }
                    }
                }
            }
        }
    }
    
    Ok(text)
}

// ============================================================================
// Main Connection
// ============================================================================

#[tauri::command]
pub async fn test_gemini_connection(
    state: tauri::State<'_, GeminiState>,
    app: AppHandle,
    key: String,
    model: Option<String>,
) -> Result<String, String> {
    *state.api_key.lock().unwrap() = Some(key.clone());
    
    let m = model.unwrap_or_else(|| state.selected_model.lock().unwrap().clone());
    *state.selected_model.lock().unwrap() = m.clone();
    
    println!("========================================");
    println!("[GEMINI] Model: {}", m);
    println!("[GEMINI] Rate limits: {}s min interval, {}s initial backoff", 
             MIN_REQUEST_INTERVAL_SECS, INITIAL_BACKOFF_SECS);
    println!("========================================");
    
    let _ = app.emit("god:status", "Testing...");
    
    // Quick test
    let url = format!("{}/{}:generateContent?key={}", GEMINI_REST_URL, m, key);
    let client = reqwest::Client::new();
    
    match client.post(&url)
        .json(&serde_json::json!({"contents":[{"parts":[{"text":"OK"}]}]}))
        .timeout(Duration::from_secs(10))
        .send().await 
    {
        Ok(r) => {
            let status = r.status();
            let t = r.text().await.unwrap_or_default();
            
            if status.as_u16() == 429 {
                println!("[GEMINI] Rate limited (429)");
                let _ = app.emit("god:status", "Rate limited");
                return Err("Rate limited".into());
            } else if status.as_u16() == 403 {
                println!("[GEMINI] Quota exhausted (403)");
                let _ = app.emit("god:status", "Quota exhausted");
                return Err("Quota exhausted".into());
            } else if !status.is_success() {
                println!("[GEMINI] HTTP error: {}", status);
                let _ = app.emit("god:status", format!("HTTP {}", status));
                return Err(format!("HTTP {}", status));
            }
            
            // Success - connected
            println!("[GEMINI] Connection test passed");
            *state.is_connected.lock().unwrap() = true;
            let _ = app.emit("god:status", "Connected ✓");
        }
        Err(e) => {
            let _ = app.emit("god:status", format!("Failed: {}", e));
            return Err(e.to_string());
        }
    }
    
    // Start smart audio loop if not already running
    let audio_rx = state.audio_rx.lock().unwrap().take();
    if let Some(rx) = audio_rx {
        let app = app.clone();
        tokio::spawn(async move {
            smart_audio_loop(rx, app).await;
        });
    }
    
    Ok(format!("Connected to {}", m))
}

#[tauri::command]
pub fn update_gemini_key(state: tauri::State<'_, GeminiState>, key: String) -> Result<(), String> {
    *state.api_key.lock().unwrap() = Some(key);
    Ok(())
}

// ============================================================================
// Smart Audio Loop with Rate Limiting
// ============================================================================

async fn smart_audio_loop(rx: Receiver<Vec<f32>>, app: AppHandle) {
    println!("[AUDIO] Loop started - Min {}s speech, {}s silence timeout", 
             MIN_SPEECH_SECS, SILENCE_TIMEOUT_SECS);
    
    let _ = app.emit("god:status", "Listening...");
    
    let mut buffer: Vec<f32> = Vec::new();
    let mut speaking = false;
    let mut speech_start: Option<Instant> = None;
    let mut last_speech: Option<Instant> = None;
    let mut processing = false;
    
    // Rate limiting state
    let mut backoff: u64 = 0;
    let mut last_request = Instant::now() - Duration::from_secs(MIN_REQUEST_INTERVAL_SECS);
    let mut request_count = 0u32;
    
    let mut tick = interval(Duration::from_millis(100));
    
    loop {
        tick.tick().await;
        
        if processing { continue; }
        
        // Collect audio
        let mut new: Vec<f32> = Vec::new();
        while let Ok(s) = rx.try_recv() { new.extend(s); }
        if new.is_empty() { continue; }
        
        let level = rms(&new);
        
        // Speech detection
        if level > SPEECH_THRESHOLD {
            if !speaking {
                speaking = true;
                speech_start = Some(Instant::now());
                println!("[AUDIO] Speech started");
            }
            last_speech = Some(Instant::now());
            buffer.extend(new);
        } else if level > SILENCE_THRESHOLD && speaking {
            buffer.extend(new);
            last_speech = Some(Instant::now());
        } else if speaking {
            buffer.extend(new);
        }
        
        // Check if should process
        let should_process = if speaking {
            let duration = speech_start.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
            let silence = last_speech.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
            
            (duration >= MIN_SPEECH_SECS && silence >= SILENCE_TIMEOUT_SECS)
                || duration >= MAX_BATCH_SECS
        } else { false };
        
        if should_process && !buffer.is_empty() {
            let duration = buffer.len() as f32 / 16000.0;
            
            if duration >= MIN_SPEECH_SECS {
                processing = true;
                request_count += 1;
                
                let _ = app.emit("god:status", format!("Processing {:.1}s (#{})...", duration, request_count));
                println!("[AUDIO] Processing {:.1}s, request #{}", duration, request_count);
                
                let audio = buffer.clone();
                buffer.clear();
                speaking = false;
                speech_start = None;
                last_speech = None;
                
                // Get current key and model from state
                let (key, model) = {
                    let state = app.state::<GeminiState>();
                    let k: String = state.api_key.lock().unwrap().clone().unwrap_or_default();
                    let m = state.selected_model.lock().unwrap().clone();
                    (k, m)
                };

                if key.is_empty() {
                    println!("[GEMINI] ✗ Error: No API key configured");
                    let _ = app.emit("god:status", "Error: No API key");
                    let _ = app.emit("god:api_error", serde_json::json!({"code": 401, "message": "No API key configured"}));
                    processing = false;
                    continue;
                }
                
                match call_gemini_with_backoff(&key, &model, &audio, &mut backoff, &mut last_request).await {
                    Ok(response) => {
                        println!("[GEMINI] ✓ Response received");
                        let _ = app.emit("god:transcript", response);
                        let _ = app.emit("god:status", "Listening...");
                    }
                    Err(e) => {
                        println!("[GEMINI] ✗ Error: {}", e);
                        let _ = app.emit("god:status", format!("Error: {}. Waiting...", e));
                        
                        // Emit error for frontend rotation
                        let code = if e.contains("429") || e.contains("Rate limit") { 429 } else { 500 };
                        let _ = app.emit("god:api_error", serde_json::json!({
                            "code": code,
                            "message": e
                        }));

                        // Extra wait on error
                        sleep(Duration::from_secs(3)).await;
                        let _ = app.emit("god:status", "Listening...");
                    }
                }
                
                processing = false;
            } else {
                println!("[AUDIO] Discarding short segment ({:.1}s)", duration);
                buffer.clear();
                speaking = false;
                speech_start = None;
                last_speech = None;
            }
        }
        
        // Prevent buffer from growing too large
        let max_samples = (MAX_BATCH_SECS * 16000.0) as usize;
        if buffer.len() > max_samples {
            buffer.drain(0..buffer.len() - max_samples);
        }
    }
}

#[tauri::command]
pub fn set_gemini_model(state: tauri::State<'_, GeminiState>, model: String) -> Result<String, String> {
    *state.selected_model.lock().unwrap() = model.clone();
    Ok(format!("Model: {}", model))
}

#[tauri::command]
pub fn get_available_models() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"id": "gemini-2.5-flash-preview-09-2025", "name": "⚡ Gemini 2.5 Flash"}),
        serde_json::json!({"id": "gemini-2.5-flash-lite-preview-09-2025", "name": "🔥 Gemini 2.5 Flash Lite"}),
        serde_json::json!({"id": "gemini-3-flash-preview", "name": "💎 Gemini 3 Flash"}),
    ]
}
