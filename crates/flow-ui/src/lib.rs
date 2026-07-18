mod notify;
mod overlay;
mod theme;

use crossbeam_channel::{Receiver, Sender};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub use notify::notify_error;
pub use overlay::{OverlayEvent, OverlayHandle};
pub use theme::apply_signal_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,
    Listening,
    Processing,
    Error,
}

static QUIT_MENU_ID: &str = "flow-quit";
static TOGGLE_MENU_ID: &str = "flow-toggle";
static SETTINGS_MENU_ID: &str = "flow-settings";

pub struct TrayHandle {
    state_tx: Sender<TrayState>,
    enabled: Arc<AtomicBool>,
    _thread: JoinHandle<()>,
}

impl TrayHandle {
    pub fn spawn() -> (Self, Receiver<()>, Receiver<()>) {
        let (state_tx, state_rx) = crossbeam_channel::unbounded();
        let (quit_tx, quit_rx) = crossbeam_channel::unbounded();
        let (settings_tx, settings_rx) = crossbeam_channel::unbounded();
        let enabled = Arc::new(AtomicBool::new(true));
        let enabled_thread = Arc::clone(&enabled);

        let thread = thread::Builder::new()
            .name("flow-tray".into())
            .spawn(move || run_tray_loop(state_rx, quit_tx, settings_tx, enabled_thread))
            .expect("failed to spawn tray thread");

        (
            Self {
                state_tx,
                enabled,
                _thread: thread,
            },
            quit_rx,
            settings_rx,
        )
    }

    pub fn set_state(&self, state: TrayState) {
        let _ = self.state_tx.send(state);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

fn run_tray_loop(
    state_rx: Receiver<TrayState>,
    quit_tx: Sender<()>,
    settings_tx: Sender<()>,
    enabled: Arc<AtomicBool>,
) {
    #[cfg(target_os = "linux")]
    if gtk::init().is_err() {
        tracing::error!("gtk init failed — tray icon unavailable");
        return;
    }

    let tray = match build_tray(&enabled) {
        Ok(tray) => tray,
        Err(e) => {
            tracing::error!(error = %e, "failed to create tray icon");
            return;
        }
    };

    let mut current = TrayState::Idle;
    update_tray_icon(&tray, current);

    loop {
        while let Ok(state) = state_rx.try_recv() {
            if state != current {
                current = state;
                update_tray_icon(&tray, current);
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = event
            {
                tracing::debug!("tray left-click");
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let id = event.id.0.as_str();
            if id == QUIT_MENU_ID {
                let _ = quit_tx.send(());
                drop(tray);
                return;
            }
            if id == SETTINGS_MENU_ID {
                let _ = settings_tx.send(());
            }
            if id == TOGGLE_MENU_ID {
                let next = !enabled.load(Ordering::Relaxed);
                enabled.store(next, Ordering::Relaxed);
                tracing::info!(enabled = next, "dictation toggled from tray");
            }
        }

        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        thread::sleep(Duration::from_millis(16));
    }
}

fn build_tray(enabled: &Arc<AtomicBool>) -> Result<TrayIcon, String> {
    let toggle = MenuItem::with_id(TOGGLE_MENU_ID, toggle_label(enabled), true, None);
    let settings = MenuItem::with_id(SETTINGS_MENU_ID, "Settings…", true, None);
    let quit = MenuItem::with_id(QUIT_MENU_ID, "Quit", true, None);
    let separator = PredefinedMenuItem::separator();
    let menu = Menu::with_items(&[&toggle, &settings, &separator, &quit])
        .map_err(|e| e.to_string())?;

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Flow Linux — hold hotkey to dictate")
        .with_icon(icon_for_state(TrayState::Idle))
        .build()
        .map_err(|e| e.to_string())
}

fn toggle_label(enabled: &Arc<AtomicBool>) -> String {
    if enabled.load(Ordering::Relaxed) {
        "Pause dictation".to_string()
    } else {
        "Resume dictation".to_string()
    }
}

fn update_tray_icon(tray: &TrayIcon, state: TrayState) {
    let _ = tray.set_icon(Some(icon_for_state(state)));
    let tooltip = match state {
        TrayState::Idle => "Flow Linux — ready",
        TrayState::Listening => "Flow Linux — listening…",
        TrayState::Processing => "Flow Linux — processing…",
        TrayState::Error => "Flow Linux — error",
    };
    let _ = tray.set_tooltip(Some(tooltip));
}

fn icon_for_state(state: TrayState) -> Icon {
    let bytes: &[u8] = match state {
        TrayState::Idle => include_bytes!("../../../assets/icons/tray-idle.png"),
        TrayState::Listening => include_bytes!("../../../assets/icons/tray-listening.png"),
        TrayState::Processing => include_bytes!("../../../assets/icons/tray-processing.png"),
        TrayState::Error => include_bytes!("../../../assets/icons/tray-error.png"),
    };
    png_to_icon(bytes).unwrap_or_else(|e| {
        tracing::error!(error = %e, "failed to decode tray icon — using fallback");
        solid_fallback(state)
    })
}

fn png_to_icon(bytes: &[u8]) -> Result<Icon, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).map_err(|e| e.to_string())
}

fn solid_fallback(state: TrayState) -> Icon {
    let (r, g, b) = match state {
        TrayState::Idle => (0x2B, 0xB8, 0xA8),
        TrayState::Listening => (0xE4, 0x57, 0x4D),
        TrayState::Processing => (0xE0, 0xA8, 0x4A),
        TrayState::Error => (0xE0, 0x70, 0x40),
    };
    let size = 22usize;
    let mut rgba = vec![0u8; size * size * 4];
    for px in rgba.chunks_exact_mut(4) {
        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = 255;
    }
    Icon::from_rgba(rgba, size as u32, size as u32).expect("valid tray icon rgba")
}
