use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use crossbeam_channel::Receiver;
use tauri::{AppHandle, Emitter};

// GOD PROMPT V8 Schema Structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntelligenceOutput {
    pub timestamp_ms: u64,
    pub speaker_id: String,
    pub transcript_chunk: String,
    pub is_final: bool,
    pub intelligence: Intelligence,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Intelligence {
    pub category: Vec<String>,
    pub summary: Option<String>,
    pub tone: Option<String>,
    pub confidence: f32,
    pub entities: Option<Vec<Entity>>,
    pub graph_updates: Option<Vec<GraphUpdate>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub text: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphUpdate {
    pub node_a: String,
    pub relation: String,
    pub node_b: String,
    pub weight: Option<f32>,
    pub directional: Option<bool>,
    pub tone_modifier: Option<f32>,
}

pub struct GeminiState {
    pub audio_rx: Mutex<Option<Receiver<Vec<f32>>>>,
    pub api_key: Mutex<Option<String>>,
}

impl Default for GeminiState {
    fn default() -> Self {
        Self {
            audio_rx: Mutex::new(None),
            api_key: Mutex::new(None),
        }
    }
}

pub async fn run_gemini_loop(
    api_key: String, 
    audio_rx: Receiver<Vec<f32>>, 
    app: AppHandle
) {
    let model = "gemini-2.0-flash-exp";
    let ws_url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}", 
        api_key
    );

    match connect_async(&ws_url).await {
        Ok((ws_stream, _)) => {
            println!("Connected to Gemini Live API");
            let _ = app.emit("god:status", "GEMINI_CONNECTED");
            
            let (mut write, mut read) = ws_stream.split();
            
            // 1. Send Setup Message
            let setup = serde_json::json!({
                "setup": {
                    "model": format!("models/{}", model),
                    "generation_config": {
                        "response_modalities": ["TEXT"],
                        "speech_config": {
                            "voice_config": {
                                "prebuilt_voice_config": {
                                    "voice_name": "Puck"
                                }
                            }
                        }
                    }
                }
            });
            
            println!("Sending setup: {}", setup);

            if let Err(e) = write.send(Message::Text(setup.to_string())).await {
                 eprintln!("Failed to send setup: {}", e);
                 let _ = app.emit("god:status", "CONNECTION_ERROR");
                 return;
            }

            // 2. Spawn Audio Sender Task
            let mut write_clone = write; 
            
            let (local_tx, mut local_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
            
            // Audio Thread -> Tokio Channel
            let audio_rx_clone = audio_rx.clone();
            std::thread::spawn(move || {
                while let Ok(chunk) = audio_rx_clone.recv() {
                    // Convert to pcm16
                     let pcm_i16: Vec<i16> = chunk.iter()
                        .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
                        .collect();
                    
                    let bytes: Vec<u8> = pcm_i16.iter()
                        .flat_map(|&sample| sample.to_le_bytes())
                        .collect();
                    
                    let encoded = base64::encode(&bytes);
                    
                    let msg = serde_json::json!({
                        "realtime_input": {
                            "media_chunks": [{
                                "mime_type": "audio/pcm",
                                "data": encoded
                            }]
                        }
                    });
                     let _ = local_tx.send(Message::Text(msg.to_string()));
                }
            });

            // Tokio Task: Combine Audio Messages + WebSocket Read
            // Using select!
            
            loop {
                tokio::select! {
                    // Send Audio
                    Some(msg) = local_rx.recv() => {
                        if let Err(e) = write_clone.send(msg).await {
                             eprintln!("Error sending audio: {}", e);
                             break;
                        }
                    }
                    // Receive Intelligence
                    Some(msg) = read.next() => {
                        match msg {
                            Ok(Message::Text(text)) => {
                                println!("Gemini Msg: {}", text); 
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                    // Handle ServerContent
                                    if let Some(server_content) = parsed.get("serverContent") {
                                        if let Some(model_turn) = server_content.get("modelTurn") {
                                             if let Some(parts) = model_turn.get("parts") {
                                                 for part in parts.as_array().unwrap_or(&vec![]) {
                                                     if let Some(text) = part.get("text") {
                                                         // We received raw text response.
                                                         let raw_text = text.as_str().unwrap_or("");
                                                         
                                                         // Emit event
                                                         let _ = app.emit("god:transcript", raw_text);
                                                         
                                                         // Try parsing as JSON for intelligence
                                                         if let Ok(intel) = serde_json::from_str::<IntelligenceOutput>(raw_text) {
                                                              let _ = app.emit("god:intelligence", intel);
                                                         }
                                                     }
                                                 }
                                             }
                                        }
                                    }
                                }
                            }
                            Ok(Message::Close(reason)) => {
                                println!("Connection closed by server: {:?}", reason);
                                let status_msg = if let Some(r) = reason {
                                    format!("DISCONNECTED: {}", r.reason)
                                } else {
                                    "DISCONNECTED".to_string()
                                };
                                let _ = app.emit("god:status", status_msg);
                                break;
                            }
                            Err(e) => {
                                eprintln!("Error receiving: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                    else => break,
                }
            }
            let _ = app.emit("god:status", "DISCONNECTED");
        }
        Err(e) => {
             eprintln!("Connect error: {}", e);
             let _ = app.emit("god:status", "CONNECTION_FAILED");
        }
    }
}

#[tauri::command]
pub async fn test_gemini_connection(
    app: AppHandle, 
    state: tauri::State<'_, GeminiState>,
    api_key: String
) -> Result<String, String> {
    // Save API key
    *state.api_key.lock().unwrap() = Some(api_key.clone());
    
    // Get Audio Rx
    let rx = {
        let lock = state.audio_rx.lock().unwrap();
        if let Some(r) = &*lock {
            r.clone()
        } else {
            return Err("Audio channel not initialized".to_string());
        }
    };

    // Spawn the loop in background
    let app_handle = app.clone();
    tokio::spawn(async move {
        run_gemini_loop(api_key, rx, app_handle).await;
    });

    Ok("Gemini loop started".to_string())
}
