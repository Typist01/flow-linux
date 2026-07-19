mod app;
mod engines;
mod pipeline;
mod state;

use app::{acquire_single_instance, init_logging};
use engines::{build_engines, EngineError, EngineSet};
use flow_audio::AudioCapture;
use flow_config::{Config, SttMode, SttProvider, SAMPLE_RATE, STREAMING_SAMPLE_RATE};
use flow_hotkey::{start as start_hotkey, HotkeyEvent};
use flow_settings::open_settings_window;
use flow_stt::{RealtimeTranscriptionSession, StreamingEvent};
use flow_ui::{notify_error, OverlayEvent, OverlayHandle, TrayHandle, TrayState};
use pipeline::{finish_transcript, run_dictation_pipeline, PipelineError};
use state::DictationPhase;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::RwLock;

struct StreamingSession {
    session: RealtimeTranscriptionSession,
    partial: String,
    samples_sent: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    init_logging(&config);
    let _lock = acquire_single_instance()?;
    validate_startup_config(&config)?;

    let (tray, quit_rx, settings_rx) = TrayHandle::spawn();
    let (reload_tx, reload_rx) = crossbeam_channel::unbounded();
    let overlay = OverlayHandle::spawn(config.general.show_overlay, reload_tx);

    let mut audio = AudioCapture::with_sample_rate(capture_sample_rate(&config))?;
    let engines = Arc::new(RwLock::new(build_engines(&config)?));
    let mut hotkey_rx = start_hotkey(engines.read().await.config.as_ref()).await?;

    let mut phase = DictationPhase::Idle;
    let mut streaming: Option<StreamingSession> = None;

    tracing::info!(
        stt = ?config.stt.provider,
        mode = ?config.stt.mode,
        polish = ?config.polish.provider,
        polish_enabled = config.polish.enabled,
        show_overlay = config.general.show_overlay,
        "Flow Linux daemon starting"
    );
    tracing::info!("Ready — hold hotkey to dictate, release to insert text");
    tray.set_state(TrayState::Idle);

    loop {
        tokio::select! {
            Some(event) = hotkey_rx.recv() => {
                if !tray.is_enabled() {
                    tracing::debug!("dictation disabled — ignoring hotkey");
                    continue;
                }

                match (phase, event) {
                    (DictationPhase::Idle, HotkeyEvent::Pressed) => {
                        let streaming_mode = engines.read().await.config.stt.is_streaming();
                        if streaming_mode {
                            match start_streaming_session(&engines, audio.sample_rate()).await {
                                Ok(session) => {
                                    audio.start_recording();
                                    streaming = Some(StreamingSession {
                                        session,
                                        partial: String::new(),
                                        samples_sent: 0,
                                    });
                                    phase = DictationPhase::Recording;
                                    tray.set_state(TrayState::Listening);
                                    overlay.send(OverlayEvent::ShowListening { streaming: true });
                                    tracing::info!("listening (streaming)...");
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "failed to start streaming session");
                                    overlay.send(OverlayEvent::Hide);
                                    notify_error(format!("Streaming failed: {e}"));
                                    tray.set_state(TrayState::Error);
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    tray.set_state(TrayState::Idle);
                                }
                            }
                        } else {
                            audio.start_recording();
                            phase = DictationPhase::Recording;
                            tray.set_state(TrayState::Listening);
                            overlay.send(OverlayEvent::ShowListening { streaming: false });
                            tracing::info!("listening...");
                        }
                    }
                    (DictationPhase::Recording, HotkeyEvent::Released) => {
                        tray.set_state(TrayState::Processing);
                        overlay.send(OverlayEvent::Processing);
                        tracing::info!("processing...");

                        if let Some(mut active) = streaming.take() {
                            let leftover = audio.stop_recording_raw();
                            if !leftover.is_empty() {
                                active.samples_sent += leftover.len();
                                if let Err(e) = active.session.append_pcm_f32(&leftover) {
                                    tracing::error!(error = %e, "failed to append final audio");
                                }
                            }

                            // OpenAI requires >= 100ms of audio before commit.
                            let min_samples = (audio.sample_rate() as usize) / 10;
                            if active.samples_sent < min_samples {
                                tracing::debug!(
                                    samples = active.samples_sent,
                                    min_samples,
                                    "skipping streaming commit — buffer too small"
                                );
                                active.session.close().await;
                                overlay.send(OverlayEvent::Hide);
                                phase = DictationPhase::Idle;
                                tray.set_state(TrayState::Idle);
                                continue;
                            }

                            let result = complete_streaming(&engines, &mut active.session).await;
                            active.session.close().await;

                            match result {
                                Ok(text) if !text.is_empty() => {
                                    tracing::info!(%text, "injected");
                                    overlay.send(OverlayEvent::Hide);
                                    tray.set_state(TrayState::Idle);
                                }
                                Ok(_) => {
                                    tracing::debug!("empty transcript");
                                    overlay.send(OverlayEvent::Hide);
                                    tray.set_state(TrayState::Idle);
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "streaming dictation failed");
                                    overlay.send(OverlayEvent::Hide);
                                    notify_error(format!("Dictation failed: {e}"));
                                    tray.set_state(TrayState::Error);
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    tray.set_state(TrayState::Idle);
                                }
                            }
                        } else {
                            let samples = match audio.stop_recording() {
                                Some(s) => s,
                                None => {
                                    phase = DictationPhase::Idle;
                                    overlay.send(OverlayEvent::Hide);
                                    tray.set_state(TrayState::Idle);
                                    continue;
                                }
                            };

                            let active = engines.read().await;
                            match run_dictation_pipeline(&active, samples).await {
                                Ok(text) if !text.is_empty() => {
                                    tracing::info!(%text, "injected");
                                    overlay.send(OverlayEvent::Hide);
                                    tray.set_state(TrayState::Idle);
                                }
                                Ok(_) => {
                                    tracing::debug!("empty transcript");
                                    overlay.send(OverlayEvent::Hide);
                                    tray.set_state(TrayState::Idle);
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "dictation pipeline failed");
                                    overlay.send(OverlayEvent::Hide);
                                    notify_error(format!("Dictation failed: {e}"));
                                    tray.set_state(TrayState::Error);
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    tray.set_state(TrayState::Idle);
                                }
                            }
                        }

                        phase = DictationPhase::Idle;
                    }
                    (DictationPhase::Recording, HotkeyEvent::Pressed) => {
                        tracing::debug!("already recording");
                    }
                    (DictationPhase::Idle, HotkeyEvent::Released) => {
                        tracing::debug!("release without press — ignored");
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(40)), if phase == DictationPhase::Recording => {
                if let Some(active) = streaming.as_mut() {
                    let chunk = audio.take_pending_samples();
                    if !chunk.is_empty() {
                        active.samples_sent += chunk.len();
                        overlay.send(OverlayEvent::Level(normalize_level(rms(&chunk))));
                        if let Err(e) = active.session.append_pcm_f32(&chunk) {
                            tracing::error!(error = %e, "failed to append streaming audio");
                        }
                    }
                    for _ in 0..8 {
                        match tokio::time::timeout(
                            Duration::from_millis(1),
                            active.session.recv_event(),
                        )
                        .await
                        {
                            Ok(Some(StreamingEvent::Delta(delta))) => {
                                tracing::info!(%delta, "streaming delta");
                                active.partial.push_str(&delta);
                                overlay.send(OverlayEvent::Partial(active.partial.clone()));
                            }
                            Ok(Some(StreamingEvent::Completed(text))) => {
                                tracing::info!(%text, "streaming segment completed");
                                if !text.is_empty() {
                                    active.partial = text;
                                    overlay.send(OverlayEvent::Partial(active.partial.clone()));
                                }
                            }
                            Ok(Some(StreamingEvent::Error(e))) => {
                                tracing::error!(error = %e, "streaming session error");
                            }
                            Ok(Some(StreamingEvent::SessionReady)) => {
                                tracing::debug!("streaming session ready");
                            }
                            Ok(None) | Err(_) => break,
                        }
                    }
                } else {
                    // Batch: peek RMS without draining the capture buffer.
                    overlay.send(OverlayEvent::Level(normalize_level(audio.recent_rms())));
                }
            }
            _ = wait_for_channel(&settings_rx) => {
                open_settings_window();
            }
            _ = wait_for_channel(&reload_rx) => {
                if phase == DictationPhase::Recording {
                    tracing::warn!("ignoring config reload while recording");
                } else {
                    match reload_engines(&engines).await {
                        Ok(config) => {
                            overlay.set_enabled(config.general.show_overlay);
                            match AudioCapture::with_sample_rate(capture_sample_rate(&config)) {
                                Ok(rebuilt) => {
                                    audio = rebuilt;
                                    tracing::info!(
                                        sample_rate = audio.sample_rate(),
                                        show_overlay = config.general.show_overlay,
                                        "reloaded engines from config"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        "engines reloaded but audio capture rebuild failed"
                                    );
                                }
                            }
                        }
                        Err(e) => tracing::error!(error = %e, "failed to reload engines"),
                    }
                }
            }
            _ = wait_for_channel(&quit_rx) => {
                tracing::info!("quit requested from tray");
                overlay.send(OverlayEvent::Hide);
                if let Some(session) = streaming.take() {
                    session.session.close().await;
                }
                break;
            }
            _ = signal::ctrl_c() => {
                tracing::info!("shutting down");
                overlay.send(OverlayEvent::Hide);
                if let Some(session) = streaming.take() {
                    session.session.close().await;
                }
                break;
            }
        }
    }

    Ok(())
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

fn normalize_level(rms: f32) -> f32 {
    // Map typical speech RMS into a 0..=1 display range.
    (rms / 0.12).clamp(0.0, 1.0)
}

async fn start_streaming_session(
    engines: &Arc<RwLock<EngineSet>>,
    source_sample_rate: u32,
) -> Result<RealtimeTranscriptionSession, flow_stt::RealtimeError> {
    let config = engines.read().await.config.clone();
    RealtimeTranscriptionSession::connect(config.as_ref(), source_sample_rate).await
}

async fn complete_streaming(
    engines: &Arc<RwLock<EngineSet>>,
    session: &mut RealtimeTranscriptionSession,
) -> Result<String, PipelineError> {
    session.commit()?;
    let raw = session.wait_for_completed(Duration::from_secs(30)).await?;
    let active = engines.read().await;
    finish_transcript(&active, raw).await
}

fn validate_startup_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if config.stt.mode == SttMode::Batch
        && config.stt.provider == SttProvider::Local
        && !flow_stt::ensure_local_model_exists(config)
    {
        tracing::error!(path = %config.model_path().display(), "whisper model not found");
        tracing::error!("Download: mkdir -p ~/.cache/flow-linux/models && curl -L -o ~/.cache/flow-linux/models/ggml-base.en.bin \\");
        tracing::error!(
            "  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
        );
        std::process::exit(1);
    }
    Ok(())
}

fn capture_sample_rate(config: &Config) -> u32 {
    if config.stt.is_streaming() {
        STREAMING_SAMPLE_RATE
    } else {
        SAMPLE_RATE
    }
}

async fn reload_engines(engines: &Arc<RwLock<EngineSet>>) -> Result<Config, EngineError> {
    let config = Config::load();
    validate_reload_config(&config)?;
    let rebuilt = build_engines(&config)?;
    *engines.write().await = rebuilt;
    Ok(config)
}

fn validate_reload_config(config: &Config) -> Result<(), EngineError> {
    if config.stt.mode == SttMode::Batch
        && config.stt.provider == SttProvider::Local
        && !flow_stt::ensure_local_model_exists(config)
    {
        return Err(EngineError::Stt(flow_stt::SttError::ModelNotFound(
            config.model_path().display().to_string(),
        )));
    }
    Ok(())
}

async fn wait_for_channel(rx: &crossbeam_channel::Receiver<()>) {
    loop {
        if rx.try_recv().is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
