use flow_config::PRE_INJECT_DELAY_MS;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

#[derive(Debug, Error)]
pub enum InjectError {
    #[error("wl-copy failed")]
    WlCopy,
    #[error("ydotool paste failed — is ydotool.service active?")]
    YdotoolPaste,
    #[error("ydotool type failed")]
    YdotoolType,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Inject text into the focused window.
/// Uses clipboard+paste by default (validated on KDE Wayland in Spike 1).
pub async fn inject_text(text: &str, via_paste: bool) -> Result<(), InjectError> {
    if text.is_empty() {
        tracing::debug!("empty text — skipping injection");
        return Ok(());
    }

    sleep(Duration::from_millis(PRE_INJECT_DELAY_MS)).await;

    if via_paste {
        inject_via_paste(text).await
    } else {
        inject_via_type(text).await
    }
}

async fn inject_via_paste(text: &str) -> Result<(), InjectError> {
    let copy_status = Command::new("wl-copy")
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !copy_status.success() {
        return Err(InjectError::WlCopy);
    }

    // Ctrl+Shift+V — KEY_LEFTCTRL=29, KEY_LEFTSHIFT=42, KEY_V=47
    let paste_status = Command::new("ydotool")
        .arg("key")
        .arg("29:1")
        .arg("42:1")
        .arg("47:1")
        .arg("47:0")
        .arg("42:0")
        .arg("29:0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !paste_status.success() {
        return Err(InjectError::YdotoolPaste);
    }

    tracing::info!(chars = text.len(), "injected via clipboard paste");
    Ok(())
}

async fn inject_via_type(text: &str) -> Result<(), InjectError> {
    let status = Command::new("ydotool")
        .arg("type")
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        tracing::warn!("ydotool type failed, falling back to paste");
        return inject_via_paste(text).await;
    }

    tracing::info!(chars = text.len(), "injected via ydotool type");
    Ok(())
}
