use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use flow_config::{SAMPLE_RATE, SILENCE_RMS_THRESHOLD};
use std::sync::atomic::{AtomicBool, Ordering};
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
    #[error("default input config: {0}")]
    DefaultConfig(#[from] cpal::DefaultStreamConfigError),
    #[error("unsupported sample format: {0:?}")]
    UnsupportedFormat(SampleFormat),
}

pub struct AudioCapture {
    _stream: cpal::Stream,
    /// Mono PCM at the device's native sample rate.
    buffer: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    device_name: String,
    /// Native capture rate (before resample).
    device_sample_rate: u32,
    /// Rate exposed to STT / callers after resampling.
    sample_rate: u32,
}

impl AudioCapture {
    pub fn new() -> Result<Self, AudioError> {
        Self::with_sample_rate(SAMPLE_RATE)
    }

    pub fn with_sample_rate(target_sample_rate: u32) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(AudioError::NoDevice)?;
        let device_name = device.name()?;

        // Use the device default config (often stereo @ 44.1/48k on PipeWire).
        // Opening a hard-coded mono 16k stream frequently yields silence on pro/USB interfaces.
        let supported = device.default_input_config()?;
        let channels = supported.channels() as usize;
        let device_sample_rate = supported.sample_rate().0;
        let sample_format = supported.sample_format();
        let stream_config: cpal::StreamConfig = supported.clone().into();

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let recording = Arc::new(AtomicBool::new(false));

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::I8 => build_stream::<i8>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::U8 => build_stream::<u8>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::I32 => build_stream::<i32>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::U32 => build_stream::<u32>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            SampleFormat::F64 => build_stream::<f64>(
                &device,
                &stream_config,
                channels,
                Arc::clone(&buffer),
                Arc::clone(&recording),
            )?,
            other => return Err(AudioError::UnsupportedFormat(other)),
        };
        stream.play()?;

        tracing::info!(
            device = %device_name,
            channels,
            device_sample_rate,
            target_sample_rate,
            ?sample_format,
            "audio capture ready"
        );

        Ok(Self {
            _stream: stream,
            buffer,
            recording,
            device_name,
            device_sample_rate,
            sample_rate: target_sample_rate,
        })
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn start_recording(&self) {
        self.buffer.lock().unwrap().clear();
        self.recording.store(true, Ordering::Relaxed);
        tracing::debug!("recording started");
    }

    /// Drain samples captured since the last call, resampled to the target rate.
    pub fn take_pending_samples(&self) -> Vec<f32> {
        let native = {
            let mut buffer = self.buffer.lock().unwrap();
            std::mem::take(&mut *buffer)
        };
        resample_linear(&native, self.device_sample_rate, self.sample_rate)
    }

    /// RMS of the most recent samples without draining (for overlay waveform in batch mode).
    pub fn recent_rms(&self) -> f32 {
        let buffer = self.buffer.lock().unwrap();
        if buffer.is_empty() {
            return 0.0;
        }
        let window = buffer.len().min(2048);
        let slice = &buffer[buffer.len() - window..];
        (slice.iter().map(|s| s * s).sum::<f32>() / slice.len() as f32).sqrt()
    }

    /// Stop recording and return all remaining samples (no silence filter), at target rate.
    pub fn stop_recording_raw(&self) -> Vec<f32> {
        self.recording.store(false, Ordering::Relaxed);
        self.take_pending_samples()
    }

    /// Stop recording and return samples if speech was detected.
    pub fn stop_recording(&self) -> Option<Vec<f32>> {
        let samples = self.stop_recording_raw();
        if samples.is_empty() {
            tracing::warn!("no samples captured");
            return None;
        }

        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        tracing::info!(samples = samples.len(), rms, "recording stopped");

        if rms < SILENCE_RMS_THRESHOLD {
            tracing::warn!(rms, "silence guard — skipping transcription");
            return None;
        }

        Some(samples)
    }

    pub fn cancel_recording(&self) {
        self.recording.store(false, Ordering::Relaxed);
        self.buffer.lock().unwrap().clear();
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    buffer: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let err_fn = |err| tracing::error!(error = %err, "audio stream error");
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            if !recording.load(Ordering::Relaxed) {
                return;
            }
            let mut out = buffer.lock().unwrap();
            push_downmixed(data, channels, &mut out);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn push_downmixed<T>(data: &[T], channels: usize, out: &mut Vec<f32>)
where
    T: Sample,
    f32: FromSample<T>,
{
    if channels <= 1 {
        out.reserve(data.len());
        for &sample in data {
            out.push(sample.to_sample::<f32>());
        }
        return;
    }

    out.reserve(data.len() / channels);
    for frame in data.chunks_exact(channels) {
        let mut sum = 0.0f32;
        for &sample in frame {
            sum += sample.to_sample::<f32>();
        }
        out.push(sum / channels as f32);
    }
}

/// Linear resample mono f32 PCM from `from_rate` to `to_rate`.
pub fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let out_len = ((samples.len() as f64) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let idx = src.floor() as usize;
        let frac = (src - idx as f64) as f32;
        let a = samples[idx.min(samples.len() - 1)];
        let b = samples[(idx + 1).min(samples.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}

/// Convert mono f32 samples to little-endian PCM16 bytes.
pub fn samples_to_pcm16_bytes(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let value = (clamped * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity_when_rates_match() {
        let samples = vec![0.0, 0.5, -0.5];
        assert_eq!(resample_linear(&samples, 16_000, 16_000), samples);
    }

    #[test]
    fn resample_16k_to_24k_grows_by_1_5x() {
        let samples = vec![0.0; 160];
        let out = resample_linear(&samples, 16_000, 24_000);
        assert_eq!(out.len(), 240);
    }

    #[test]
    fn pcm16_bytes_are_little_endian() {
        let bytes = samples_to_pcm16_bytes(&[1.0, -1.0]);
        // 1.0 → i16::MAX (0x7FFF); -1.0 → -32767 (0x8001) due to cast from f32
        assert_eq!(bytes, [0xff, 0x7f, 0x01, 0x80]);
    }

    #[test]
    fn downmix_stereo_averages_channels() {
        let data = [0.0f32, 1.0, 0.5, 0.5];
        let mut out = Vec::new();
        push_downmixed(&data, 2, &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.5).abs() < f32::EPSILON);
        assert!((out[1] - 0.5).abs() < f32::EPSILON);
    }
}
