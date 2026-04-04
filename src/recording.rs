use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Returns the names of all available input devices on the default host.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub struct RecordingEngine {
    is_recording: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    _stream: Option<cpal::Stream>,
}

impl RecordingEngine {
    pub fn new() -> Self {
        RecordingEngine {
            is_recording: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 44100,
            channels: 1,
            _stream: None,
        }
    }

    /// Returns true if currently recording.
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    /// Start capturing from the named input device, or the default if `device_name` is `None`.
    /// Returns false if no suitable device is available.
    pub fn start(&mut self, device_name: Option<&str>) -> bool {
        let host = cpal::default_host();
        let device = if let Some(name) = device_name {
            // Try to find the named device; fall back to default.
            host.input_devices()
                .ok()
                .and_then(|mut devs| devs.find(|d| d.name().ok().as_deref() == Some(name)))
                .or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        };
        let device = match device {
            Some(d) => d,
            None => return false,
        };
        let supported_config = match device.default_input_config() {
            Ok(c) => c,
            Err(_) => return false,
        };

        self.sample_rate = supported_config.sample_rate().0;
        self.channels = supported_config.channels();

        // Clear any previous recording data
        self.samples.lock().unwrap().clear();

        let samples_clone = Arc::clone(&self.samples);
        let is_rec_clone = Arc::clone(&self.is_recording);
        let stream_config: cpal::StreamConfig = supported_config.config();

        let stream = match supported_config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if is_rec_clone.load(Ordering::Relaxed) {
                        samples_clone.lock().unwrap().extend_from_slice(data);
                    }
                },
                |err| eprintln!("cpal input error: {err}"),
                None,
            ),
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if is_rec_clone.load(Ordering::Relaxed) {
                            let mut guard = samples_clone.lock().unwrap();
                            for &s in data {
                                guard.push(s as f32 / 32768.0);
                            }
                        }
                    },
                    |err| eprintln!("cpal input error: {err}"),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if is_rec_clone.load(Ordering::Relaxed) {
                            let mut guard = samples_clone.lock().unwrap();
                            for &s in data {
                                guard.push((s as f32 - 32768.0) / 32768.0);
                            }
                        }
                    },
                    |err| eprintln!("cpal input error: {err}"),
                    None,
                )
            }
            _ => return false,
        };

        match stream {
            Ok(s) => {
                if s.play().is_err() {
                    return false;
                }
                self.is_recording.store(true, Ordering::Relaxed);
                self._stream = Some(s);
                true
            }
            Err(_) => false,
        }
    }

    /// Stop recording and return a WAV-encoded buffer of the captured audio.
    pub fn stop(&mut self) -> Vec<u8> {
        self.is_recording.store(false, Ordering::Relaxed);
        self._stream = None; // dropping the stream halts capture

        let samples = self.samples.lock().unwrap();
        encode_wav(&samples, self.sample_rate, self.channels)
    }
}

/// Encode f32 samples as a 16-bit PCM WAV buffer.
fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    let pcm: Vec<i16> = samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();

    let data_size = (pcm.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_size as usize);

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format tag
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * 2;
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for s in pcm {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}
