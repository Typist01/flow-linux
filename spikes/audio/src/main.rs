//! Spike 3: Validate cpal hold-to-record PCM capture.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

const SAMPLE_RATE: u32 = 16_000;
const SILENCE_RMS_THRESHOLD: f32 = 0.005;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().without_time().init();

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device found")?;
    tracing::info!(device = %device.name()?, "Using input device");

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = Arc::clone(&buffer);

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            buffer_clone.lock().unwrap().extend_from_slice(data);
        },
        |e| tracing::error!(error = %e, "Stream error"),
        None,
    )?;
    stream.play()?;

    println!("Spike 3: type 'start' to record, 'stop' to finish, 'quit' to exit.");
    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        stdin.read_line(&mut line)?;
        match line.trim() {
            "start" => {
                buffer.lock().unwrap().clear();
                tracing::info!("Recording... speak now, then type 'stop'");
            }
            "stop" => {
                let samples = buffer.lock().unwrap().clone();
                if samples.is_empty() {
                    tracing::warn!("No samples captured");
                    continue;
                }
                let rms =
                    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
                tracing::info!(samples = samples.len(), rms, "Captured");
                if rms < SILENCE_RMS_THRESHOLD {
                    tracing::warn!("Silence guard triggered — no speech detected");
                    continue;
                }
                let path = "spike-output.wav";
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: SAMPLE_RATE,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                let mut writer = hound::WavWriter::create(path, spec)?;
                for &s in &samples {
                    let amp = (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    writer.write_sample(amp)?;
                }
                writer.finalize()?;
                tracing::info!(path, "WAV written — play it back to verify your voice");
            }
            "quit" => break,
            _ => println!("Commands: start, stop, quit"),
        }
    }
    Ok(())
}
