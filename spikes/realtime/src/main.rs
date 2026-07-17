//! Spike: OpenAI Realtime transcription (gpt-realtime-whisper).
//!
//! Press Enter to start capturing, speak, press Enter again to commit.
//! Requires OPENAI_API_KEY or a key stored in the Flow Linux keyring.

use flow_audio::AudioCapture;
use flow_config::{Config, SttMode, SttProvider, STREAMING_SAMPLE_RATE};
use flow_stt::{RealtimeTranscriptionSession, StreamingEvent};
use std::io::{self, BufRead, Write};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .without_time()
        .init();

    let mut config = Config::load();
    config.stt.mode = SttMode::Streaming;
    config.stt.provider = SttProvider::Openai;
    if config.stt.streaming_model.is_empty() {
        config.stt.streaming_model = "gpt-realtime-whisper".into();
    }
    if config.stt.streaming_delay.is_empty() {
        config.stt.streaming_delay = "low".into();
    }

    tracing::info!(
        model = %config.stt.streaming_model,
        delay = %config.stt.streaming_delay,
        "connecting realtime transcription session"
    );
    let mut session = RealtimeTranscriptionSession::connect(&config, STREAMING_SAMPLE_RATE).await?;

    wait_for_enter("Press Enter to start recording")?;
    let audio = AudioCapture::with_sample_rate(STREAMING_SAMPLE_RATE)?;
    audio.start_recording();
    tracing::info!("recording — press Enter to stop and commit");

    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::task::spawn_blocking(move || {
        let _ = wait_for_enter("");
        let _ = stop_tx.send(());
    });

    loop {
        tokio::select! {
            _ = &mut stop_rx => break,
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                let chunk = audio.take_pending_samples();
                if !chunk.is_empty() {
                    session.append_pcm_f32(&chunk)?;
                }
                while let Ok(Some(event)) = tokio::time::timeout(
                    Duration::from_millis(1),
                    session.recv_event(),
                ).await {
                    match event {
                        StreamingEvent::Delta(delta) => {
                            print!("{delta}");
                            let _ = io::stdout().flush();
                        }
                        StreamingEvent::Completed(text) => {
                            println!("\n[completed] {text}");
                        }
                        StreamingEvent::Error(e) => {
                            tracing::error!(error = %e, "realtime error");
                        }
                        StreamingEvent::SessionReady => {}
                    }
                }
            }
        }
    }

    let leftover = audio.stop_recording_raw();
    if !leftover.is_empty() {
        session.append_pcm_f32(&leftover)?;
    }

    println!();
    tracing::info!("committing audio buffer...");
    session.commit()?;
    let transcript = session.wait_for_completed(Duration::from_secs(30)).await?;
    println!("\nFINAL: {transcript}");
    session.close().await;
    Ok(())
}

fn wait_for_enter(prompt: &str) -> io::Result<()> {
    if !prompt.is_empty() {
        println!("{prompt}");
    }
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(())
}
