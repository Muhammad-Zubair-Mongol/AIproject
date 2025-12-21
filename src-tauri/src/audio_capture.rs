use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

pub struct AudioState {
    pub is_recording: Mutex<bool>,
    pub stream_control: Mutex<Option<Sender<()>>>,
    pub audio_tx: Mutex<Option<Sender<Vec<f32>>>>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            is_recording: Mutex::new(false),
            stream_control: Mutex::new(None),
            audio_tx: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let mut names = Vec::new();

    names.push("--- Input Devices ---".to_string());
    if let Ok(devices) = host.input_devices() {
        for device in devices {
             if let Ok(name) = device.name() {
                names.push(name);
            }
        }
    }

    names.push("--- Output Devices ---".to_string());
    if let Ok(devices) = host.output_devices() {
        for device in devices {
             if let Ok(name) = device.name() {
                names.push(name);
            }
        }
    }
    
    Ok(names)
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}

// Simple Mono Mixer and Resampler wrapper
struct AudioProcessor {
    resampler: Option<SincFixedIn<f32>>,
    input_sample_rate: usize,
    output_sample_rate: usize,
    chunk_buffer: Vec<f32>,
}

impl AudioProcessor {
    fn new(input_rate: usize, output_rate: usize, chunk_size: usize) -> Self {
        let resampler = if input_rate != output_rate {
            // High quality resampling parameters
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            Some(SincFixedIn::<f32>::new(
                output_rate as f64 / input_rate as f64,
                256.0, // max_resample_ratio_relative
                params,
                chunk_size, 
                1 // channels
            ).unwrap())
        } else {
            None
        };

        Self {
            resampler,
            input_sample_rate: input_rate,
            output_sample_rate: output_rate,
            chunk_buffer: Vec::with_capacity(chunk_size * 2),
        }
    }

    fn process(&mut self, data: &[f32], channels: u16) -> Vec<f32> {
        // 1. Valid mono downmix
        let mono_data: Vec<f32> = list_to_mono(data, channels);

        // 2. Resample if needed
        if let Some(resampler) = &mut self.resampler {
            let waves_in = vec![mono_data];
            // rubato expects Vec<Vec<f32>>
            match resampler.process(&waves_in, None) {
                Ok(waves_out) => waves_out[0].clone(),
                Err(e) => {
                    eprintln!("Resample error: {}", e);
                    vec![] // Failed
                }
            }
        } else {
            mono_data
        }
    }
}

fn list_to_mono(data: &[f32], channels: u16) -> Vec<f32> {
    data.chunks(channels as usize)
        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
        .collect()
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
    
    // Get the audio transmitter if it exists
    let audio_tx = {
        let tx_lock = state.audio_tx.lock().map_err(|e| e.to_string())?;
        tx_lock.clone()
    };

    if audio_tx.is_none() {
        // Just warning, maybe we want to run without Gemini connected?
        println!("WARNING: No audio transmitter connected. Audio will be captured but not sent.");
    }

    thread::spawn(move || {
        let host = cpal::default_host();
        // Gemini expects 16kHz
        let target_sample_rate = 16000;
        
        // --- MICROPHONE SETUP ---
        let input_device = host.default_input_device();
        let input_stream = if let Some(device) = input_device {
            println!("Input device: {}", device.name().unwrap_or_default());
            let config = device.default_input_config().unwrap();
            let input_channels = config.channels();
            let input_rate = config.sample_rate().0;
            
            println!("Input config: {}ch, {}Hz", input_channels, input_rate);
            
            let tx = audio_tx.clone();
            let err_fn = move |err| eprintln!("Input stream error: {}", err);
            
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                     if !data.is_empty() {
                         let mono = list_to_mono(data, input_channels);
                         // Note: Resampling should happen here or in downstream consumer.
                         // For MVP, if input is 48kHz and we send to Gemini 16kHz, we must decimate.
                         // Naive decimation for common 48->16 (factor of 3) check?
                         
                         let sent_data = if input_rate == 48000 {
                             mono.into_iter().step_by(3).collect()
                         } else if input_rate == 44100 {
                             // Complex, just send raw for now and hope Gemini is robust or rubato is added later
                             mono
                         } else {
                             mono
                         };

                         if let Some(tx) = &tx {
                             let _ = tx.send(sent_data);
                         }
                     }
                },
                err_fn,
                None 
            ).ok();
            stream
        } else {
            None
        };

        if let Some(s) = &input_stream { s.play().unwrap(); }

        println!("Streams playing...");
        let _ = stop_rx.recv();
        println!("Streams stopping...");
    });

    *is_rec = true;
    Ok("Capture started".to_string())
}

#[tauri::command]
pub fn stop_audio_capture(state: tauri::State<'_, AudioState>) -> Result<String, String> {
    let mut is_rec = state.is_recording.lock().map_err(|e| e.to_string())?;
    if !*is_rec {
        return Ok("Not recording".to_string());
    } // stop control
    
    let mut control = state.stream_control.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = control.take() {
        let _ = tx.send(());
    }

    *is_rec = false;
    Ok("Capture stopped".to_string())
}
