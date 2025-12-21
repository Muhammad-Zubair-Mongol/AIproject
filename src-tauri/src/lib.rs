mod audio_capture;
mod gemini_client;
mod processing_engine;
mod session_manager;
use audio_capture::AudioState;
use gemini_client::GeminiState;
use std::sync::Mutex;
use crossbeam_channel::unbounded;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    Emitter, Manager,
};

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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Create tray menu
            let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let record_i = MenuItem::with_id(app, "record", "Start Recording", true, None::<&str>)?;
            let stop_i = MenuItem::with_id(app, "stop", "Stop Recording", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&show_i, &record_i, &stop_i, &quit_i])?;
            
            // Build tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Meeting Mind - Intelligence Engine")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "record" => {
                            println!("[TRAY] Start recording triggered");
                            // Emit event to frontend
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("tray:record", ());
                            }
                        }
                        "stop" => {
                            println!("[TRAY] Stop recording triggered");
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("tray:stop", ());
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            
            println!("[STATION 6] Tray icon initialized - Shadow mode ready");
            
            Ok(())
        })
        .manage(audio_state)
        .manage(gemini_state)
        .invoke_handler(tauri::generate_handler![
            greet, 
            audio_capture::list_audio_devices,
            audio_capture::start_audio_capture,
            audio_capture::stop_audio_capture,
            audio_capture::set_capture_mode,
            audio_capture::get_current_volume,
            gemini_client::test_gemini_connection,
            gemini_client::set_gemini_model,
            gemini_client::get_available_models,
            processing_engine::validate_json_schema,
            processing_engine::update_processing_settings,
            processing_engine::get_recent_intelligence,
            processing_engine::clear_intelligence_cache,
            processing_engine::inject_manual_intelligence,
            session_manager::save_session,
            session_manager::load_session,
            session_manager::list_sessions,
            session_manager::delete_session,
            session_manager::export_session,
            session_manager::generate_session_summary,
            session_manager::get_session_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
