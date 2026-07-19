use ashpd::desktop::global_shortcuts::{
    BindShortcutsOptions, GlobalShortcuts, ListShortcutsOptions, NewShortcut,
};
use ashpd::desktop::CreateSessionOptions;
use ashpd::AppID;
use flow_config::Config;
use futures::StreamExt;
use std::str::FromStr;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Pressed,
    Released,
}

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("portal: {0}")]
    Portal(#[from] ashpd::Error),
    #[error("invalid app id")]
    InvalidAppId,
    #[error("shortcut not bound — assign a key in KDE Settings or run scripts/fix-spike-hotkey-binding.sh")]
    ShortcutUnbound,
}

fn is_unbound(trigger: &str) -> bool {
    let t = trigger.trim().to_lowercase();
    t.is_empty() || t == "none" || t.contains("not set") || t.contains("no shortcut")
}

/// Start global hotkey listener. Returns a channel of press/release events.
pub async fn start(config: &Config) -> Result<mpsc::UnboundedReceiver<HotkeyEvent>, HotkeyError> {
    let app_id = AppID::from_str(config.app_id()).map_err(|_| HotkeyError::InvalidAppId)?;
    if let Err(e) = ashpd::register_host_app(app_id).await {
        tracing::error!(error = %e, "portal registration failed");
        tracing::error!("Run: ./scripts/install-spike-hotkey-desktop.sh");
        tracing::error!(
            "Desktop file must exist at ~/.local/share/applications/{}.desktop",
            config.desktop_file_name()
        );
        return Err(e.into());
    }

    let global_shortcuts = GlobalShortcuts::new().await?;
    let session = global_shortcuts
        .create_session(CreateSessionOptions::default())
        .await?;

    let shortcut = NewShortcut::new(config.shortcut_id(), "Flow Linux dictation")
        .preferred_trigger(Some(config.hotkey_trigger()));

    let bind_request = global_shortcuts
        .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
        .await?;
    bind_request.response()?;

    let list_request = global_shortcuts
        .list_shortcuts(&session, ListShortcutsOptions::default())
        .await?;
    let list_response = list_request.response()?;

    let mut bound_trigger = None;
    for s in list_response.shortcuts() {
        let trigger = s.trigger_description();
        tracing::info!(id = s.id(), trigger, "active shortcut");
        if !is_unbound(trigger) {
            bound_trigger = Some(trigger.to_string());
        }
    }

    if bound_trigger.is_none() {
        return Err(HotkeyError::ShortcutUnbound);
    }

    tracing::info!(
        trigger = bound_trigger.as_deref().unwrap_or("unknown"),
        "hotkey listener ready — hold to record, release to transcribe"
    );

    let (tx, rx) = mpsc::unbounded_channel();

    let mut activated = global_shortcuts.receive_activated().await?;
    let mut deactivated = global_shortcuts.receive_deactivated().await?;

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(_event) = activated.next() => {
                    let _ = tx.send(HotkeyEvent::Pressed);
                }
                Some(_event) = deactivated.next() => {
                    let _ = tx.send(HotkeyEvent::Released);
                }
                else => break,
            }
        }
    });

    Ok(rx)
}
