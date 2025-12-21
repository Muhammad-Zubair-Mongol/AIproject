use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::time::{Duration, interval, timeout, Instant};
use crossbeam_channel::Receiver;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;

// ============================================================================
// GEMINI CLIENT - Smart Audio Processing with GOD PROMPT V9
// ============================================================================

const GEMINI_REST_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

// SMART PROCESSING CONFIG
const MIN_SPEECH_DURATION_SECS: f32 = 2.0;  // Minimum 2 seconds of speech
const MAX_BATCH_DURATION_SECS: f32 = 10.0;  // Maximum 10 seconds per batch  
const SILENCE_THRESHOLD: f32 = 0.015;       // RMS below this = silence
const SPEECH_THRESHOLD: f32 = 0.02;         // RMS above this = speech
const SILENCE_TIMEOUT_SECS: f32 = 1.5;      // 1.5 seconds of silence = end of utterance

type WsWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ApiMode {
    RestApi,
}

pub struct GeminiState {
    pub audio_rx: StdMutex<Option<Receiver<Vec<f32>>>>,
    pub api_key: StdMutex<Option<String>>,
    pub is_connected: StdMutex<bool>,
    pub selected_model: StdMutex<String>,
    pub api_mode: StdMutex<ApiMode>,
    pub ws_write: Mutex<Option<Arc<Mutex<WsWrite>>>>,
}

impl Default for GeminiState {
    fn default() -> Self {
        Self {
            audio_rx: StdMutex::new(None),
            api_key: StdMutex::new(None),
            is_connected: StdMutex::new(false),
            selected_model: StdMutex::new("gemini-2.5-flash-preview-09-2025".to_string()),
            api_mode: StdMutex::new(ApiMode::RestApi),
            ws_write: Mutex::new(None),
        }
    }
}

// GOD PROMPT V9 - The core system instruction
const GOD_PROMPT_V9: &str = r#"You are a PASSIVE MEETING INTELLIGENCE ENGINE. Your role is to listen and extract intelligence from meeting audio.

STRICT OUTPUT FORMAT - JSON ONLY:
{
  "transcript": "exact transcription of speech",
  "speaker": "Speaker 1",
  "tone": "NEUTRAL|URGENT|FRUSTRATED|EXCITED|POSITIVE|NEGATIVE|HESITANT|DOMINANT|EMPATHETIC",
  "category": ["TASK", "DECISION", "DEADLINE", "QUERY", "ACTION_ITEM", "RISK", "INFO"],
  "confidence": 0.85,
  "language": "en|ur|hi"
}

RULES:
1. Output ONLY valid JSON - no markdown, no explanations
2. Transcribe speech accurately, preserve language (English, Urdu, Hindi)
3. Detect speaker tone from voice patterns
4. Categorize content appropriately
5. Set confidence based on audio clarity
6. If audio is unclear or silence: {"status": "silence", "confidence": 0.0}

Be concise. Extract intelligence. No small talk."#;

// ============================================================================
// REST API Structures
// ============================================================================

#[derive(Serialize)]
struct RestRequest {
    contents: Vec<Content>,
    system_instruction: Option<SystemInstruction>,
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineData>,
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<TextPart>,
}

#[derive(Serialize)]
struct TextPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: i32,
}

#[derive(Deserialize, Debug)]
struct RestResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize, Debug)]
struct CandidateContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Deserialize, Debug)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    message: Option<String>,
}

// ============================================================================
// Audio Processing Helpers
// ============================================================================

fn samples_to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let byte_rate = sample_rate * 2;
    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;
    
    let mut wav = Vec::with_capacity(44 + num_samples * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    
    for sample in samples {
        let s = (*sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        wav.extend_from_slice(&s.to_le_bytes());
    }
    wav
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

async fn send_to_gemini(key: &str, model: &str, audio: &[f32]) -> Result<String, String> {
    let wav_data = samples_to_wav(audio, 16000);
    let audio_base64 = BASE64.encode(&wav_data);
    
    let request = RestRequest {
        contents: vec![Content {
            parts: vec![
                Part {
                    text: Some("Analyze this meeting audio and extract intelligence:".to_string()),
                    inline_data: None,
                },
                Part {
                    text: None,
                    inline_data: Some(InlineData {
                        mime_type: "audio/wav".to_string(),
                        data: audio_base64,
                    }),
                },
            ],
        }],
        system_instruction: Some(SystemInstruction {
            parts: vec![TextPart { text: GOD_PROMPT_V9.to_string() }],
        }),
        generation_config: Some(GenerationConfig {
            temperature: 0.1,
            max_output_tokens: 1024,
        }),
    };
    
    let url = format!("{}/{}:generateContent?key={}", GEMINI_REST_URL, model, key);
    
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    
    let response_text = response.text().await.map_err(|e| format!("Read error: {}", e))?;
    
    if let Ok(resp) = serde_json::from_str::<RestResponse>(&response_text) {
        if let Some(error) = resp.error {
            return Err(format!("API error: {}", error.message.unwrap_or_default()));
        }
        if let Some(candidates) = resp.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(content) = &candidate.content {
                    if let Some(parts) = &content.parts {
                        if let Some(part) = parts.first() {
                            if let Some(text) = &part.text {
                                return Ok(text.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Return raw response if parsing fails
    Ok(response_text)
}

// ============================================================================
// Main Connection Command
// ============================================================================

#[tauri::command]
pub async fn test_gemini_connection(
    state: tauri::State<'_, GeminiState>,
    app: AppHandle,
    key: String,
    model: Option<String>,
) -> Result<String, String> {
    *state.api_key.lock().unwrap() = Some(key.clone());
    
    let selected_model = if let Some(m) = &model {
        *state.selected_model.lock().unwrap() = m.clone();
        m.clone()
    } else {
        state.selected_model.lock().unwrap().clone()
    };
    
    println!("========================================");
    println!("[GEMINI] Model: {}", selected_model);
    println!("[GEMINI] Using REST API with smart batching");
    println!("========================================");
    
    let _ = app.emit("god:status", "Testing connection...");
    
    // Test with simple request
    let test_url = format!("{}/{}:generateContent?key={}", GEMINI_REST_URL, selected_model, key);
    let test_req = serde_json::json!({
        "contents": [{"parts": [{"text": "Reply with only: OK"}]}]
    });
    
    let client = reqwest::Client::new();
    match client.post(&test_url).json(&test_req).timeout(Duration::from_secs(10)).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.contains("error") {
                println!("[GEMINI] Error response: {}", &text[..text.len().min(200)]);
                let _ = app.emit("god:status", "API Error - Check key");
                return Err("API error".to_string());
            }
            println!("[GEMINI] Connection test passed");
            *state.is_connected.lock().unwrap() = true;
            let _ = app.emit("god:status", "Connected ✓");
        }
        Err(e) => {
            let _ = app.emit("god:status", format!("Failed: {}", e));
            return Err(format!("Connection failed: {}", e));
        }
    }
    
    // Start smart audio processing
    let audio_rx = state.audio_rx.lock().unwrap().take();
    if let Some(rx) = audio_rx {
        let app_clone = app.clone();
        let key_clone = key.clone();
        let model_clone = selected_model.clone();
        
        tokio::spawn(async move {
            smart_audio_loop(rx, app_clone, key_clone, model_clone).await;
        });
    }
    
    Ok(format!("Connected to {}", selected_model))
}

// ============================================================================
// Smart Audio Processing Loop
// ============================================================================

async fn smart_audio_loop(
    rx: Receiver<Vec<f32>>,
    app: AppHandle,
    key: String,
    model: String,
) {
    println!("[AUDIO] Smart processing loop started");
    println!("[AUDIO] Min speech: {}s, Silence timeout: {}s", MIN_SPEECH_DURATION_SECS, SILENCE_TIMEOUT_SECS);
    
    let _ = app.emit("god:status", "Listening...");
    
    let mut speech_buffer: Vec<f32> = Vec::new();
    let mut is_speaking = false;
    let mut speech_start: Option<Instant> = None;
    let mut last_speech: Option<Instant> = None;
    let mut is_processing = false;
    
    let mut tick = interval(Duration::from_millis(100));
    let sample_rate = 16000;
    
    loop {
        tick.tick().await;
        
        // Skip if already processing
        if is_processing {
            continue;
        }
        
        // Collect all available audio
        let mut new_samples: Vec<f32> = Vec::new();
        while let Ok(samples) = rx.try_recv() {
            new_samples.extend(samples);
        }
        
        if new_samples.is_empty() {
            continue;
        }
        
        // Calculate RMS for this chunk
        let rms = calculate_rms(&new_samples);
        
        // Speech detection state machine
        if rms > SPEECH_THRESHOLD {
            // Speech detected
            if !is_speaking {
                is_speaking = true;
                speech_start = Some(Instant::now());
                println!("[AUDIO] Speech started (RMS: {:.3})", rms);
            }
            last_speech = Some(Instant::now());
            speech_buffer.extend(new_samples);
        } else if rms > SILENCE_THRESHOLD {
            // Borderline - keep buffering if we're already speaking
            if is_speaking {
                speech_buffer.extend(new_samples);
                last_speech = Some(Instant::now());
            }
        } else {
            // Silence
            if is_speaking {
                speech_buffer.extend(new_samples);
            }
        }
        
        // Check if we should process
        let should_process = if is_speaking {
            let speech_duration = speech_start.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
            let silence_duration = last_speech.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
            
            // Process if:
            // 1. We have enough speech AND silence indicates end of utterance
            // 2. OR we hit max batch duration
            let end_of_utterance = speech_duration >= MIN_SPEECH_DURATION_SECS 
                                   && silence_duration >= SILENCE_TIMEOUT_SECS;
            let max_reached = speech_duration >= MAX_BATCH_DURATION_SECS;
            
            end_of_utterance || max_reached
        } else {
            false
        };
        
        if should_process && !speech_buffer.is_empty() {
            let speech_duration = speech_buffer.len() as f32 / sample_rate as f32;
            
            // Only process if we have minimum duration
            if speech_duration >= MIN_SPEECH_DURATION_SECS {
                is_processing = true;
                let _ = app.emit("god:status", format!("Processing {:.1}s...", speech_duration));
                
                println!("[AUDIO] Processing {:.1}s of speech ({} samples)", speech_duration, speech_buffer.len());
                
                let audio_to_process = speech_buffer.clone();
                let app_clone = app.clone();
                let key_clone = key.clone();
                let model_clone = model.clone();
                
                // Reset state before async call
                speech_buffer.clear();
                is_speaking = false;
                speech_start = None;
                last_speech = None;
                
                // Process async
                match send_to_gemini(&key_clone, &model_clone, &audio_to_process).await {
                    Ok(response) => {
                        println!("[GEMINI] Response: {}", &response[..response.len().min(200)]);
                        let _ = app_clone.emit("god:transcript", response);
                        let _ = app_clone.emit("god:status", "Listening...");
                    }
                    Err(e) => {
                        println!("[GEMINI] Error: {}", e);
                        let _ = app_clone.emit("god:status", format!("Error: {}", e));
                        // Wait a bit before retrying
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        let _ = app_clone.emit("god:status", "Listening...");
                    }
                }
                
                is_processing = false;
            } else {
                // Not enough speech, discard
                println!("[AUDIO] Discarding short speech ({:.1}s < {}s)", speech_duration, MIN_SPEECH_DURATION_SECS);
                speech_buffer.clear();
                is_speaking = false;
                speech_start = None;
                last_speech = None;
            }
        }
        
        // Prevent buffer from growing too large during speech
        let max_buffer_samples = (MAX_BATCH_DURATION_SECS * sample_rate as f32) as usize;
        if speech_buffer.len() > max_buffer_samples {
            speech_buffer.drain(0..speech_buffer.len() - max_buffer_samples);
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
        serde_json::json!({
            "id": "gemini-2.5-flash-preview-09-2025",
            "name": "⚡ Gemini 2.5 Flash",
            "mode": "REST"
        }),
        serde_json::json!({
            "id": "gemini-2.5-flash-lite-preview-09-2025",
            "name": "🔥 Gemini 2.5 Flash Lite",
            "mode": "REST"
        }),
        serde_json::json!({
            "id": "gemini-3-flash-preview",
            "name": "💎 Gemini 3 Flash",
            "mode": "REST"
        }),
    ]
}
