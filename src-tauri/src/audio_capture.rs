use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};

// Audio state for Tauri
pub struct AudioState {
    pub is_recording: Mutex<bool>,
    pub stream_control: Mutex<Option<Sender<()>>>,
    pub audio_tx: Mutex<Option<Sender<Vec<f32>>>>,
    pub current_volume: Arc<Mutex<f32>>,
    pub capture_mode: Mutex<CaptureMode>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CaptureMode {
    MicOnly,
    SystemOnly,  // WASAPI Loopback on Windows
    Both,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            is_recording: Mutex::new(false),
            stream_control: Mutex::new(None),
            audio_tx: Mutex::new(None),
            current_volume: Arc::new(Mutex::new(0.0)),
            capture_mode: Mutex::new(CaptureMode::Both), // Default to Both (Mic + System)
        }
    }
}

const TARGET_SAMPLE_RATE: u32 = 16000;
const MICRO_CHUNK_SAMPLES: usize = 160;
const SILENCE_THRESHOLD: f32 = 0.01;
const SILENCE_SKIP_CHUNKS: usize = 30;

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    
    // Default host devices
    let host = cpal::default_host();
    names.push(format!("Host: {}", host.id().name()));
    
    names.push("--- Microphones ---".to_string());
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                names.push(format!("  🎤 {}", name));
            }
        }
    }
    
    names.push("--- Output Devices ---".to_string());
    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                names.push(format!("  🔊 {}", name));
            }
        }
    }
    
    // Check WASAPI availability (Windows only)
    #[cfg(target_os = "windows")]
    {
        names.push("--- WASAPI Loopback ---".to_string());
        names.push("  ✓ Available for System Audio Capture".to_string());
    }
    
    Ok(names)
}

#[tauri::command]
pub fn set_capture_mode(state: tauri::State<'_, AudioState>, mode: String) -> Result<String, String> {
    let mut capture_mode = state.capture_mode.lock().map_err(|e| e.to_string())?;
    let new_mode = match mode.as_str() {
        "mic" => CaptureMode::MicOnly,
        "system" => CaptureMode::SystemOnly,
        "both" => CaptureMode::Both,
        _ => return Err("Invalid mode".to_string()),
    };
    *capture_mode = new_mode;
    println!("[AUDIO] Capture mode: {:?}", new_mode);
    Ok(format!("Mode: {:?}", new_mode))
}

#[tauri::command]
pub fn get_current_volume(state: tauri::State<'_, AudioState>) -> Result<f32, String> {
    let volume = state.current_volume.lock().map_err(|e| e.to_string())?;
    Ok(*volume)
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

fn to_mono(data: &[f32], channels: u16) -> Vec<f32> {
    data.chunks(channels as usize)
        .map(|ch| ch.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn decimate(samples: Vec<f32>, from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate { return samples; }
    let factor = from_rate / to_rate;
    if factor == 0 { return samples; }
    samples.into_iter().step_by(factor as usize).collect()
}

#[tauri::command]
pub fn start_audio_capture(state: tauri::State<'_, AudioState>) -> Result<String, String> {
    let mut is_rec = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_rec {
        return Ok("Already recording".to_string());
    }

    let (stop_tx, stop_rx) = unbounded::<()>();
    {
        let mut control = state.stream_control.lock().map_err(|e| e.to_string())?;
        *control = Some(stop_tx);
    }
    
    let audio_tx = state.audio_tx.lock().map_err(|e| e.to_string())?.clone();
    let capture_mode = *state.capture_mode.lock().map_err(|e| e.to_string())?;
    let volume = state.current_volume.clone();

    println!("[AUDIO] Starting capture. Mode: {:?}", capture_mode);

    thread::spawn(move || {
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let silence_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        
        // === MICROPHONE CAPTURE ===
        let mic_stream = if capture_mode == CaptureMode::MicOnly || capture_mode == CaptureMode::Both {
            let host = cpal::default_host();
            if let Some(device) = host.default_input_device() {
                let name = device.name().unwrap_or_default();
                println!("[AUDIO] Mic: {}", name);
                
                if let Ok(config) = device.default_input_config() {
                    let channels = config.channels();
                    let sample_rate = config.sample_rate().0;
                    
                    let tx = audio_tx.clone();
                    let buf = buffer.clone();
                    let sil = silence_count.clone();
                    let vol = volume.clone();
                    
                    let stream = device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _| {
                            if data.is_empty() { return; }
                            
                            let mono = to_mono(data, channels);
                            let resampled = decimate(mono, sample_rate, TARGET_SAMPLE_RATE);
                            
                            let rms = calculate_rms(&resampled);
                            if let Ok(mut v) = vol.lock() { *v = rms; }
                            
                            // Silence detection
                            if let Ok(mut count) = sil.lock() {
                                if rms < SILENCE_THRESHOLD {
                                    *count += 1;
                                    if *count > SILENCE_SKIP_CHUNKS { return; }
                                } else {
                                    *count = 0;
                                }
                            }
                            
                            // Buffer and send chunks
                            if let Ok(mut b) = buf.lock() {
                                b.extend(resampled);
                                while b.len() >= MICRO_CHUNK_SAMPLES {
                                    let chunk: Vec<f32> = b.drain(..MICRO_CHUNK_SAMPLES).collect();
                                    if let Some(ref tx) = tx {
                                        let _ = tx.send(chunk);
                                    }
                                }
                            }
                        },
                        |e| eprintln!("[AUDIO] Mic error: {}", e),
                        None
                    ).ok();
                    stream
                } else { None }
            } else { None }
        } else { None };
        
        // === SYSTEM AUDIO (WASAPI LOOPBACK) - Windows Only ===
        #[cfg(target_os = "windows")]
        let loopback_stream = if capture_mode == CaptureMode::SystemOnly || capture_mode == CaptureMode::Both {
            use cpal::available_hosts;
            
            println!("[AUDIO] Attempting system audio capture...");
            
            // Try WASAPI host first
            let wasapi_host = available_hosts()
                .into_iter()
                .find(|h| h.name().contains("WASAPI"))
                .and_then(|id| cpal::host_from_id(id).ok());
            
            let host = wasapi_host.unwrap_or_else(|| {
                println!("[AUDIO] WASAPI not available, using default host");
                cpal::default_host()
            });
            
            // Strategy 1: Try loopback from default output device
            let loopback_result = host.default_output_device().and_then(|device| {
                let name = device.name().unwrap_or_default();
                println!("[AUDIO] Trying loopback on output device: {}", name);
                
                device.default_output_config().ok().and_then(|config| {
                    let channels = config.channels();
                    let sample_rate = config.sample_rate().0;
                    
                    let tx = audio_tx.clone();
                    let buf = buffer.clone();
                    let sil = silence_count.clone();
                    let vol = volume.clone();
                    
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _| {
                            if data.is_empty() { return; }
                            
                            let mono = to_mono(data, channels);
                            let resampled = decimate(mono, sample_rate, TARGET_SAMPLE_RATE);
                            
                            let rms = calculate_rms(&resampled);
                            if let Ok(mut v) = vol.lock() { *v = rms; }
                            
                            if let Ok(mut count) = sil.lock() {
                                if rms < SILENCE_THRESHOLD {
                                    *count += 1;
                                    if *count > SILENCE_SKIP_CHUNKS { return; }
                                } else {
                                    *count = 0;
                                }
                            }
                            
                            if let Ok(mut b) = buf.lock() {
                                b.extend(resampled);
                                while b.len() >= MICRO_CHUNK_SAMPLES {
                                    let chunk: Vec<f32> = b.drain(..MICRO_CHUNK_SAMPLES).collect();
                                    if let Some(ref tx) = tx {
                                        let _ = tx.send(chunk);
                                    }
                                }
                            }
                        },
                        |e| eprintln!("[AUDIO] Loopback stream error: {}", e),
                        None
                    ).ok()
                })
            });
            
            if loopback_result.is_some() {
                println!("[AUDIO] ✓ WASAPI loopback stream created successfully");
                loopback_result
            } else {
                // Strategy 2: Try finding Stereo Mix or similar virtual device
                println!("[AUDIO] Loopback failed, searching for Stereo Mix...");
                
                let stereo_mix = host.input_devices().ok().and_then(|devices| {
                    devices.into_iter().find(|d| {
                        if let Ok(name) = d.name() {
                            let name_lower = name.to_lowercase();
                            name_lower.contains("stereo mix") ||
                            name_lower.contains("what u hear") ||
                            name_lower.contains("wave out") ||
                            name_lower.contains("loopback")
                        } else {
                            false
                        }
                    })
                });
                
                if let Some(device) = stereo_mix {
                    let name = device.name().unwrap_or_default();
                    println!("[AUDIO] Found virtual capture device: {}", name);
                    
                    device.default_input_config().ok().and_then(|config| {
                        let channels = config.channels();
                        let sample_rate = config.sample_rate().0;
                        
                        let tx = audio_tx.clone();
                        let buf = buffer.clone();
                        let sil = silence_count.clone();
                        let vol = volume.clone();
                        
                        device.build_input_stream(
                            &config.into(),
                            move |data: &[f32], _| {
                                if data.is_empty() { return; }
                                
                                let mono = to_mono(data, channels);
                                let resampled = decimate(mono, sample_rate, TARGET_SAMPLE_RATE);
                                
                                let rms = calculate_rms(&resampled);
                                if let Ok(mut v) = vol.lock() { *v = rms; }
                                
                                if let Ok(mut count) = sil.lock() {
                                    if rms < SILENCE_THRESHOLD {
                                        *count += 1;
                                        if *count > SILENCE_SKIP_CHUNKS { return; }
                                    } else {
                                        *count = 0;
                                    }
                                }
                                
                                if let Ok(mut b) = buf.lock() {
                                    b.extend(resampled);
                                    while b.len() >= MICRO_CHUNK_SAMPLES {
                                        let chunk: Vec<f32> = b.drain(..MICRO_CHUNK_SAMPLES).collect();
                                        if let Some(ref tx) = tx {
                                            let _ = tx.send(chunk);
                                        }
                                    }
                                }
                            },
                            |e| eprintln!("[AUDIO] Stereo Mix error: {}", e),
                            None
                        ).ok()
                    })
                } else {
                    eprintln!("[AUDIO] ✗ System audio capture not available");
                    eprintln!("[AUDIO] To enable system audio capture:");
                    eprintln!("  1. Right-click Sound icon in system tray → Sounds");
                    eprintln!("  2. Go to Recording tab");
                    eprintln!("  3. Right-click → Show Disabled Devices");
                    eprintln!("  4. Enable 'Stereo Mix' if available");
                    None
                }
            }
        } else { None };
        
        #[cfg(not(target_os = "windows"))]
        let loopback_stream: Option<cpal::Stream> = None;
        
        // Play streams
        if let Some(ref s) = mic_stream { 
            if s.play().is_ok() {
                println!("[AUDIO] ✓ Mic stream active");
            }
        }
        
        #[cfg(target_os = "windows")]
        if let Some(ref s) = loopback_stream { 
            if s.play().is_ok() {
                println!("[AUDIO] ✓ Loopback stream active");
            }
        }
        
        println!("[AUDIO] Capture running...");
        let _ = stop_rx.recv();
        println!("[AUDIO] Capture stopped");
    });

    *is_rec = true;
    Ok("Capture started".to_string())
}

#[tauri::command]
pub fn stop_audio_capture(state: tauri::State<'_, AudioState>) -> Result<String, String> {
    let mut is_rec = state.is_recording.lock().map_err(|e| e.to_string())?;
    if !*is_rec {
        return Ok("Not recording".to_string());
    }
    
    let mut control = state.stream_control.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = control.take() {
        let _ = tx.send(());
    }

    *is_rec = false;
    Ok("Stopped".to_string())
}
