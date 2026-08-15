//! Spike 2: Validate GlobalShortcuts portal press/release on KDE Wayland.

use ashpd::desktop::global_shortcuts::{
    BindShortcutsOptions, ConfigureShortcutsOptions, GlobalShortcuts, ListShortcutsOptions,
    NewShortcut,
};
use ashpd::desktop::CreateSessionOptions;
use ashpd::AppID;
use futures::StreamExt;
use std::str::FromStr;

const APP_ID: &str = "io.github.Typist01.FlowLinux";
const SHORTCUT_ID: &str = "flow-dictation";
/// KDE/Qt format — Meta = Windows/Super key. Override with FLOW_SPIKE_HOTKEY env var.
const DEFAULT_TRIGGER: &str = "Meta+Ctrl+Space";

fn preferred_trigger() -> String {
    std::env::var("FLOW_SPIKE_HOTKEY").unwrap_or_else(|_| DEFAULT_TRIGGER.to_string())
}

fn is_unbound(trigger: &str) -> bool {
    let t = trigger.trim().to_lowercase();
    t.is_empty() || t == "none" || t.contains("not set") || t.contains("no shortcut")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .without_time()
        .init();

    let app_id = AppID::from_str(APP_ID)?;
    if let Err(e) = ashpd::register_host_app(app_id).await {
        tracing::error!(error = %e, "Portal app registration failed");
        tracing::error!("Run: ./scripts/install-hotkey-desktop.sh");
        return Err(e.into());
    }
    tracing::info!(app_id = APP_ID, "Registered host app with portal");

    let trigger = preferred_trigger();
    let global_shortcuts = GlobalShortcuts::new().await?;
    let session = global_shortcuts
        .create_session(CreateSessionOptions::default())
        .await?;

    let shortcut = NewShortcut::new(SHORTCUT_ID, "Flow Linux dictation")
        .preferred_trigger(Some(trigger.as_str()));

    tracing::info!(trigger = %trigger, "Binding shortcut (KDE dialog may appear)...");

    let bind_request = global_shortcuts
        .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
        .await?;
    let bind_response = bind_request.response()?;

    for s in bind_response.shortcuts() {
        tracing::info!(
            id = s.id(),
            description = s.description(),
            trigger = s.trigger_description(),
            "Bound shortcut"
        );
    }

    let list_request = global_shortcuts
        .list_shortcuts(&session, ListShortcutsOptions::default())
        .await?;
    let list_response = list_request.response()?;

    let mut any_unbound = false;
    for s in list_response.shortcuts() {
        let trigger_desc = s.trigger_description();
        tracing::info!(id = s.id(), trigger = trigger_desc, "Active shortcut");
        if is_unbound(trigger_desc) {
            any_unbound = true;
        }
    }

    if any_unbound {
        tracing::warn!("Shortcut has NO key assigned — this is why you see no signals.");
        tracing::warn!("Opening KDE shortcut configuration...");
        global_shortcuts
            .configure_shortcuts(&session, None, ConfigureShortcutsOptions::default())
            .await?;
        tracing::warn!(
            "In the dialog: assign a key to 'Flow Linux dictation' (try Meta+Ctrl+Space)."
        );
        tracing::warn!("Or run: ./scripts/fix-hotkey-binding.sh");
        tracing::warn!("Then press the assigned key while this program is still running.");
    }

    let mut activated = global_shortcuts.receive_activated().await?;
    let mut deactivated = global_shortcuts.receive_deactivated().await?;
    let mut changed = global_shortcuts.receive_shortcuts_changed().await?;

    tracing::info!("Listening for hotkey events. Ctrl+C to quit.");
    tracing::info!("Press the key shown in 'Active shortcut' above (NOT unassigned/none).");

    loop {
        tokio::select! {
            Some(event) = activated.next() => {
                tracing::info!(?event, "ACTIVATED (pressed)");
            }
            Some(event) = deactivated.next() => {
                tracing::info!(?event, "DEACTIVATED (released)");
            }
            Some(event) = changed.next() => {
                tracing::info!(?event, "Shortcuts changed — try the new binding");
            }
            _ = tokio::signal::ctrl_c() => break,
        }
    }

    tracing::info!("Spike 2 finished.");
    Ok(())
}
