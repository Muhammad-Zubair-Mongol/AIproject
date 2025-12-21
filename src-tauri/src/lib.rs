mod audio_capture;
mod gemini_client;
mod processing_engine;
mod session_manager;
use audio_capture::AudioState;
use gemini_client::GeminiState;
use std::sync::Mutex;
use crossbeam_channel::unbounded;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (audio_tx, audio_rx) = unbounded::<Vec<f32>>();

    let audio_state = AudioState {
        audio_tx: Mutex::new(Some(audio_tx)),
        ..Default::default()
    };

    let gemini_state = GeminiState {
        audio_rx: Mutex::new(Some(audio_rx)),
        ..Default::default()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(audio_state)
        .manage(gemini_state)
        .invoke_handler(tauri::generate_handler![
            greet, 
            audio_capture::list_audio_devices,
            audio_capture::start_audio_capture,
            audio_capture::stop_audio_capture,
            gemini_client::test_gemini_connection,
            processing_engine::validate_json_schema,
            session_manager::save_session,
            session_manager::load_session,
            session_manager::list_sessions,
            session_manager::delete_session,
            session_manager::export_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
