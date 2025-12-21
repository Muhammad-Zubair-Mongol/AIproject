use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};

// Audio state for Tauri
pub struct AudioState {
    pub is_recording: Mutex<bool>,
    pub stream_control: Mutex<Option<Sender<()>>>,
    pub audio_tx: Mutex<Option<Sender<Vec<f32>>>>,
    pub current_volume: Arc<Mutex<f32>>,  // Arc for thread sharing
    pub capture_mode: Mutex<CaptureMode>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CaptureMode {
    MicOnly,
    SystemOnly,
    Both,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            is_recording: Mutex::new(false),
            stream_control: Mutex::new(None),
            audio_tx: Mutex::new(None),
            current_volume: Arc::new(Mutex::new(0.0)),
            capture_mode: Mutex::new(CaptureMode::MicOnly),
        }
    }
}

// Constants for Station 1 compliance
const TARGET_SAMPLE_RATE: u32 = 16000;
const MICRO_CHUNK_SAMPLES: usize = 160; // ~10ms at 16kHz = 160 samples
const SILENCE_THRESHOLD: f32 = 0.01; // RMS below this = silence
const SILENCE_SKIP_CHUNKS: usize = 30; // Skip after 30 silent chunks (~300ms)

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let mut names = Vec::new();

    names.push("--- Input Devices (Microphone) ---".to_string());
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                names.push(format!("[MIC] {}", name));
            }
        }
    }

    names.push("--- Output Devices (System Audio Loopback) ---".to_string());
    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                names.push(format!("[SYS] {}", name));
            }
        }
    }
    
    Ok(names)
}

#[tauri::command]
pub fn set_capture_mode(state: tauri::State<'_, AudioState>, mode: String) -> Result<String, String> {
    let mut capture_mode = state.capture_mode.lock().map_err(|e| e.to_string())?;
    *capture_mode = match mode.as_str() {
        "mic" => CaptureMode::MicOnly,
        "system" => CaptureMode::SystemOnly,
        "both" => CaptureMode::Both,
        _ => return Err("Invalid mode. Use: mic, system, or both".to_string()),
    };
    Ok(format!("Capture mode set to: {}", mode))
}

#[tauri::command]
pub fn get_current_volume(state: tauri::State<'_, AudioState>) -> Result<f32, String> {
    let volume = state.current_volume.lock().map_err(|e| e.to_string())?;
    Ok(*volume)
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

fn list_to_mono(data: &[f32], channels: u16) -> Vec<f32> {
    data.chunks(channels as usize)
        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
        .collect()
}

// Decimate audio from input rate to 16kHz
fn decimate_to_16k(samples: Vec<f32>, input_rate: u32) -> Vec<f32> {
    if input_rate == TARGET_SAMPLE_RATE {
        return samples;
    }
    
    let factor = input_rate / TARGET_SAMPLE_RATE;
    if factor == 0 {
        return samples;
    }
    
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
    
    let audio_tx = {
        let tx_lock = state.audio_tx.lock().map_err(|e| e.to_string())?;
        tx_lock.clone()
    };

    let capture_mode = {
        let mode = state.capture_mode.lock().map_err(|e| e.to_string())?;
        *mode
    };

    // Clone the volume Arc for thread sharing
    let volume_for_thread = state.current_volume.clone();

    thread::spawn(move || {
        let host = cpal::default_host();
        
        // Accumulator for micro-chunking
        let chunk_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(MICRO_CHUNK_SAMPLES * 4)));
        let silence_counter: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        
        // --- MICROPHONE STREAM ---
        let mic_stream = if capture_mode == CaptureMode::MicOnly || capture_mode == CaptureMode::Both {
            if let Some(device) = host.default_input_device() {
                println!("[STATION 1] Mic device: {}", device.name().unwrap_or_default());
                let config = device.default_input_config().unwrap();
                let channels = config.channels();
                let sample_rate = config.sample_rate().0;
                
                let tx = audio_tx.clone();
                let buffer = chunk_buffer.clone();
                let silence = silence_counter.clone();
                let vol = volume_for_thread.clone();
                
                let stream = device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        if data.is_empty() { return; }
                        
                        // Mono downmix
                        let mono = list_to_mono(data, channels);
                        
                        // Decimate to 16kHz
                        let resampled = decimate_to_16k(mono, sample_rate);
                        
                        // Calculate volume for UI
                        let rms = calculate_rms(&resampled);
                        if let Ok(mut v) = vol.lock() {
                            *v = rms;
                        }
                        
                        // Silence detection
                        let is_silent = rms < SILENCE_THRESHOLD;
                        
                        if let Ok(mut count) = silence.lock() {
                            if is_silent {
                                *count += 1;
                                if *count > SILENCE_SKIP_CHUNKS {
                                    return; // Skip this chunk
                                }
                            } else {
                                *count = 0; // Reset on speech
                            }
                        }
                        
                        // Micro-chunking: accumulate and send in small chunks
                        if let Ok(mut buf) = buffer.lock() {
                            buf.extend(resampled);
                            
                            // Send micro-chunks
                            while buf.len() >= MICRO_CHUNK_SAMPLES {
                                let chunk: Vec<f32> = buf.drain(..MICRO_CHUNK_SAMPLES).collect();
                                if let Some(ref tx) = tx {
                                    let _ = tx.send(chunk);
                                }
                            }
                        }
                    },
                    |err| eprintln!("[STATION 1] Mic error: {}", err),
                    None
                ).ok();
                stream
            } else {
                None
            }
        } else {
            None
        };

        // --- SYSTEM AUDIO LOOPBACK STREAM (Windows WASAPI) ---
        let loopback_stream = if capture_mode == CaptureMode::SystemOnly || capture_mode == CaptureMode::Both {
            // On Windows, we can use output device with loopback
            // Note: This requires running as admin or proper audio permissions
            if let Some(device) = host.default_output_device() {
                println!("[STATION 1] System audio device: {}", device.name().unwrap_or_default());
                
                // Try to build an input stream from the output device (loopback)
                if let Ok(config) = device.default_output_config() {
                    let channels = config.channels();
                    let sample_rate = config.sample_rate().0;
                    
                    let tx = audio_tx.clone();
                    let buffer = chunk_buffer.clone();
                    let silence = silence_counter.clone();
                    
                    // Attempt loopback - this may fail on some systems
                    let stream = device.build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &_| {
                            if data.is_empty() { return; }
                            
                            let mono = list_to_mono(data, channels);
                            let resampled = decimate_to_16k(mono, sample_rate);
                            
                            let rms = calculate_rms(&resampled);
                            let is_silent = rms < SILENCE_THRESHOLD;
                            
                            if let Ok(mut count) = silence.lock() {
                                if is_silent {
                                    *count += 1;
                                    if *count > SILENCE_SKIP_CHUNKS {
                                        return;
                                    }
                                } else {
                                    *count = 0;
                                }
                            }
                            
                            if let Ok(mut buf) = buffer.lock() {
                                buf.extend(resampled);
                                while buf.len() >= MICRO_CHUNK_SAMPLES {
                                    let chunk: Vec<f32> = buf.drain(..MICRO_CHUNK_SAMPLES).collect();
                                    if let Some(ref tx) = tx {
                                        let _ = tx.send(chunk);
                                    }
                                }
                            }
                        },
                        |err| eprintln!("[STATION 1] Loopback error: {}", err),
                        None
                    ).ok();
                    stream
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Play streams
        if let Some(ref s) = mic_stream { 
            if let Err(e) = s.play() {
                eprintln!("[STATION 1] Failed to start mic: {}", e);
            } else {
                println!("[STATION 1] Mic stream playing");
            }
        }
        if let Some(ref s) = loopback_stream { 
            if let Err(e) = s.play() {
                eprintln!("[STATION 1] Failed to start loopback: {}", e);
            } else {
                println!("[STATION 1] Loopback stream playing");
            }
        }

        println!("[STATION 1] Audio capture active. Waiting for stop signal...");
        let _ = stop_rx.recv();
        println!("[STATION 1] Audio capture stopped.");
    });

    *is_rec = true;
    Ok("Capture started with Station 1 features".to_string())
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
    Ok("Capture stopped".to_string())
}
