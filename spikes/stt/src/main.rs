//! Spike 4: Validate whisper-rs builds and transcribes a WAV fixture.

use std::path::PathBuf;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn model_path() -> PathBuf {
    let cache = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    cache.join("flow-linux/models/ggml-base.en.bin")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().without_time().init();

    let model = model_path();
    if !model.exists() {
        eprintln!("Model not found at {}", model.display());
        eprintln!(
            "Download: mkdir -p ~/.cache/flow-linux/models && curl -L -o {} \\",
            model.display()
        );
        eprintln!("  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin");
        std::process::exit(1);
    }

    let fixture = std::env::args().nth(1).unwrap_or_else(|| "spike-output.wav".to_string());
    if !std::path::Path::new(&fixture).exists() {
        eprintln!("WAV fixture not found: {fixture}");
        eprintln!("Run Spike 3 first to create spike-output.wav, or pass a path argument.");
        std::process::exit(1);
    }

    tracing::info!(model = %model.display(), fixture, "Loading model...");
    let start = std::time::Instant::now();

    let ctx = WhisperContext::new_with_params(
        model.to_str().unwrap(),
        WhisperContextParameters::default(),
    )?;
    let mut state = ctx.create_state()?;

    let mut reader = hound::WavReader::open(&fixture)?;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_timestamps(false);

    state.full(params, &samples)?;

    let n = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n {
        if let Some(segment) = state.get_segment(i) {
            text.push_str(segment.to_str()?);
        }
    }

    let elapsed = start.elapsed();
    tracing::info!(elapsed_ms = elapsed.as_millis(), "Transcription complete");
    println!("Transcript: {}", text.trim());

    if elapsed.as_secs() > 3 {
        tracing::warn!("Took >3s — acceptable for MVP but note for Phase 3 GPU work");
    }

    Ok(())
}
