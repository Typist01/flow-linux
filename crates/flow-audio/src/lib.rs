use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use flow_config::{SAMPLE_RATE, SILENCE_RMS_THRESHOLD};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("no input device found")]
    NoDevice,
    #[error("cpal: {0}")]
    Cpal(#[from] cpal::BuildStreamError),
    #[error("cpal play: {0}")]
    Play(#[from] cpal::PlayStreamError),
    #[error("device name: {0}")]
    DeviceName(#[from] cpal::DeviceNameError),
}

pub struct AudioCapture {
    _stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    device_name: String,
}

impl AudioCapture {
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(AudioError::NoDevice)?;
        let device_name = device.name()?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                buffer_clone.lock().unwrap().extend_from_slice(data);
            },
            |err| tracing::error!(error = %err, "audio stream error"),
            None,
        )?;
        stream.play()?;

        tracing::info!(device = %device_name, "audio capture ready");

        Ok(Self {
            _stream: stream,
            buffer,
            device_name,
        })
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn start_recording(&self) {
        self.buffer.lock().unwrap().clear();
        tracing::debug!("recording started");
    }

    /// Stop recording and return samples if speech was detected.
    pub fn stop_recording(&self) -> Option<Vec<f32>> {
        let samples = self.buffer.lock().unwrap().clone();
        if samples.is_empty() {
            tracing::debug!("no samples captured");
            return None;
        }

        let rms =
            (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        tracing::debug!(samples = samples.len(), rms, "recording stopped");

        if rms < SILENCE_RMS_THRESHOLD {
            tracing::debug!("silence guard — skipping transcription");
            return None;
        }

        Some(samples)
    }
}
