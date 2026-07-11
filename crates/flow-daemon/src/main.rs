mod app;
mod engines;
mod pipeline;
mod state;

use app::{acquire_single_instance, init_logging};
use engines::{build_engines, EngineError, EngineSet};
use flow_audio::AudioCapture;
use flow_config::{Config, SttProvider};
use flow_hotkey::{start as start_hotkey, HotkeyEvent};
use flow_settings::{open_settings_window, start_settings_service};
use flow_ui::{TrayHandle, TrayState};
use pipeline::run_dictation_pipeline;
use state::DictationPhase;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    init_logging(&config);
    let _lock = acquire_single_instance()?;
    validate_startup_config(&config)?;

    let (tray, quit_rx, settings_rx) = TrayHandle::spawn();
    let (reload_tx, reload_rx) = crossbeam_channel::unbounded();
    let _settings_service = start_settings_service(reload_tx);

    let audio = AudioCapture::new()?;
    let engines = Arc::new(RwLock::new(build_engines(&config)?));
    let mut hotkey_rx = start_hotkey(engines.read().await.config.as_ref()).await?;

    let mut phase = DictationPhase::Idle;

    tracing::info!(
        stt = ?config.stt.provider,
        polish = ?config.polish.provider,
        polish_enabled = config.polish.enabled,
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
                        audio.start_recording();
                        phase = DictationPhase::Recording;
                        tray.set_state(TrayState::Listening);
                        tracing::info!("listening...");
                    }
                    (DictationPhase::Recording, HotkeyEvent::Released) => {
                        tray.set_state(TrayState::Processing);
                        tracing::info!("processing...");

                        let samples = match audio.stop_recording() {
                            Some(s) => s,
                            None => {
                                phase = DictationPhase::Idle;
                                tray.set_state(TrayState::Idle);
                                continue;
                            }
                        };

                        let active = engines.read().await;
                        match run_dictation_pipeline(&active, samples).await {
                            Ok(text) if !text.is_empty() => {
                                tracing::info!(%text, "injected");
                                tray.set_state(TrayState::Idle);
                            }
                            Ok(_) => {
                                tracing::debug!("empty transcript");
                                tray.set_state(TrayState::Idle);
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "dictation pipeline failed");
                                tray.set_state(TrayState::Error);
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                tray.set_state(TrayState::Idle);
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
            _ = wait_for_channel(&settings_rx) => {
                open_settings_window();
            }
            _ = wait_for_channel(&reload_rx) => {
                match reload_engines(&engines).await {
                    Ok(()) => tracing::info!("reloaded engines from config"),
                    Err(e) => tracing::error!(error = %e, "failed to reload engines"),
                }
            }
            _ = wait_for_channel(&quit_rx) => {
                tracing::info!("quit requested from tray");
                break;
            }
            _ = signal::ctrl_c() => {
                tracing::info!("shutting down");
                break;
            }
        }
    }

    Ok(())
}

fn validate_startup_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if config.stt.provider == SttProvider::Local && !flow_stt::ensure_local_model_exists(config) {
        tracing::error!(path = %config.model_path().display(), "whisper model not found");
        tracing::error!("Download: mkdir -p ~/.cache/flow-linux/models && curl -L -o ~/.cache/flow-linux/models/ggml-base.en.bin \\");
        tracing::error!("  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin");
        std::process::exit(1);
    }
    Ok(())
}

async fn reload_engines(engines: &Arc<RwLock<EngineSet>>) -> Result<(), EngineError> {
    let config = Config::load();
    validate_reload_config(&config)?;
    let rebuilt = build_engines(&config)?;
    *engines.write().await = rebuilt;
    Ok(())
}

fn validate_reload_config(config: &Config) -> Result<(), EngineError> {
    if config.stt.provider == SttProvider::Local && !flow_stt::ensure_local_model_exists(config) {
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
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
