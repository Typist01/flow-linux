use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use flow_settings::{register_settings_opener, SettingsApp};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const OVERLAY_WIDTH: f32 = 420.0;
const OVERLAY_HEIGHT: f32 = 56.0;
const BAR_COUNT: usize = 10;

#[derive(Debug, Clone)]
pub enum OverlayEvent {
    ShowListening { streaming: bool },
    Partial(String),
    Level(f32),
    Processing,
    Hide,
}

enum UiCommand {
    Overlay(OverlayEvent),
    OpenSettings,
}

pub struct OverlayHandle {
    cmd_tx: Sender<UiCommand>,
    enabled: Arc<AtomicBool>,
    _thread: JoinHandle<()>,
}

impl OverlayHandle {
    pub fn spawn(enabled: bool, reload_tx: Sender<()>) -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<UiCommand>();
        let (settings_open_tx, settings_open_rx) = crossbeam_channel::unbounded::<()>();
        register_settings_opener(settings_open_tx);

        let enabled = Arc::new(AtomicBool::new(enabled));
        let enabled_thread = Arc::clone(&enabled);
        let cmd_tx_forward = cmd_tx.clone();

        let _forward = thread::Builder::new()
            .name("flow-settings-forward".into())
            .spawn(move || {
                while settings_open_rx.recv().is_ok() {
                    let _ = cmd_tx_forward.send(UiCommand::OpenSettings);
                }
            })
            .expect("failed to spawn settings forwarder");

        let thread = thread::Builder::new()
            .name("flow-ui".into())
            .spawn(move || run_ui_loop(cmd_rx, enabled_thread, reload_tx))
            .expect("failed to spawn UI thread");

        Self {
            cmd_tx,
            enabled,
            _thread: thread,
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            let _ = self.cmd_tx.send(UiCommand::Overlay(OverlayEvent::Hide));
        }
    }

    pub fn send(&self, event: OverlayEvent) {
        if matches!(event, OverlayEvent::Hide) || self.enabled.load(Ordering::Relaxed) {
            let _ = self.cmd_tx.send(UiCommand::Overlay(event));
        }
    }
}

fn run_ui_loop(cmd_rx: Receiver<UiCommand>, enabled: Arc<AtomicBool>, reload_tx: Sender<()>) {
    // Wayland ignores always-on-top. Force X11/XWayland for this event loop so the
    // listening pill can stay above other windows on KDE without requiring focus.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([OVERLAY_WIDTH, OVERLAY_HEIGHT])
            .with_min_inner_size([OVERLAY_WIDTH, OVERLAY_HEIGHT])
            .with_max_inner_size([OVERLAY_WIDTH, OVERLAY_HEIGHT])
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_resizable(false)
            .with_taskbar(false)
            .with_visible(false)
            .with_active(false)
            .with_mouse_passthrough(true)
            .with_window_type(egui::X11WindowType::Notification)
            .with_title("Flow Linux"),
        centered: false,
        run_and_return: false,
        event_loop_builder: Some(Box::new(|builder| {
            #[cfg(target_os = "linux")]
            {
                use winit::platform::x11::EventLoopBuilderExtX11;
                EventLoopBuilderExtX11::with_x11(builder);
                EventLoopBuilderExtX11::with_any_thread(builder, true);
            }
        })),
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "Flow Linux",
        options,
        Box::new(move |_cc| Ok(Box::new(UiApp::new(cmd_rx, enabled, reload_tx)))),
    ) {
        tracing::error!(%error, "UI event loop failed");
    }
}

struct UiApp {
    cmd_rx: Receiver<UiCommand>,
    enabled: Arc<AtomicBool>,
    reload_tx: Sender<()>,
    visible: bool,
    streaming: bool,
    processing: bool,
    partial: String,
    bars: [f32; BAR_COUNT],
    bar_phase: usize,
    settings_open: bool,
    settings: Option<SettingsApp>,
}

impl UiApp {
    fn new(cmd_rx: Receiver<UiCommand>, enabled: Arc<AtomicBool>, reload_tx: Sender<()>) -> Self {
        Self {
            cmd_rx,
            enabled,
            reload_tx,
            visible: false,
            streaming: false,
            processing: false,
            partial: String::new(),
            bars: [0.08; BAR_COUNT],
            bar_phase: 0,
            settings_open: false,
            settings: None,
        }
    }

    fn drain_commands(&mut self, ctx: &egui::Context) {
        let mut changed = false;
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            changed = true;
            match cmd {
                UiCommand::OpenSettings => {
                    if self.settings.is_none() {
                        self.settings = Some(SettingsApp::new(self.reload_tx.clone()));
                    }
                    self.settings_open = true;
                }
                UiCommand::Overlay(event) => match event {
                    OverlayEvent::ShowListening { streaming } => {
                        if !self.enabled.load(Ordering::Relaxed) {
                            continue;
                        }
                        self.visible = true;
                        self.streaming = streaming;
                        self.processing = false;
                        self.partial.clear();
                        self.bars = [0.08; BAR_COUNT];
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            egui::WindowLevel::AlwaysOnTop,
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
                        position_top_center(ctx);
                    }
                    OverlayEvent::Partial(text) => {
                        self.partial = text;
                    }
                    OverlayEvent::Level(level) => {
                        self.push_bar(level.clamp(0.0, 1.0));
                    }
                    OverlayEvent::Processing => {
                        self.processing = true;
                        self.visible = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            egui::WindowLevel::AlwaysOnTop,
                        ));
                    }
                    OverlayEvent::Hide => {
                        self.visible = false;
                        self.processing = false;
                        self.partial.clear();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                },
            }
        }
        if changed {
            ctx.request_repaint();
        }
    }

    fn push_bar(&mut self, level: f32) {
        let scaled = (level * 8.0).clamp(0.08, 1.0);
        self.bars[self.bar_phase % BAR_COUNT] = scaled;
        self.bar_phase = self.bar_phase.wrapping_add(1);
    }

    fn show_settings_viewport(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let mut still_open = true;
        let reload_tx = self.reload_tx.clone();
        if self.settings.is_none() {
            self.settings = Some(SettingsApp::new(reload_tx));
        }

        let Some(settings) = self.settings.as_mut() else {
            return;
        };

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("flow_settings"),
            egui::ViewportBuilder::default()
                .with_title("Flow Linux Settings")
                .with_inner_size([520.0, 620.0])
                .with_min_inner_size([420.0, 480.0])
                .with_active(true)
                .with_decorations(true)
                .with_taskbar(true)
                .with_window_level(egui::WindowLevel::Normal)
                .with_mouse_passthrough(false),
            |ctx, class| {
                assert!(
                    class == egui::viewport::ViewportClass::Immediate,
                    "unexpected viewport class"
                );

                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        settings.ui(ui);
                    });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        self.settings_open = still_open;
        if !still_open {
            self.settings = None;
        }
    }
}

impl eframe::App for UiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_commands(ctx);
        self.show_settings_viewport(ctx);

        ctx.request_repaint_after(Duration::from_millis(if self.visible || self.settings_open {
            33
        } else {
            50
        }));

        if !self.visible {
            return;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(28, 28, 32, 230))
                    .corner_radius(18.0)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let status = if self.processing {
                            "Processing…"
                        } else if self.streaming {
                            "Listening (live)"
                        } else {
                            "Listening…"
                        };
                        ui.label(
                            egui::RichText::new(status)
                                .size(12.0)
                                .color(egui::Color32::from_rgb(230, 90, 90)),
                        );

                        let display = if self.processing {
                            if self.partial.is_empty() {
                                "Finalizing…".to_string()
                            } else {
                                truncate_middle(&self.partial, 52)
                            }
                        } else if self.partial.is_empty() {
                            if self.streaming {
                                "Speak…".to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            truncate_middle(&self.partial, 52)
                        };

                        if !display.is_empty() {
                            ui.label(
                                egui::RichText::new(display)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(235, 235, 240)),
                            );
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        draw_bars(ui, &self.bars);
                    });
                });
            });
    }
}

fn draw_bars(ui: &mut egui::Ui, bars: &[f32; BAR_COUNT]) {
    let height = 28.0;
    let width = 4.0;
    let gap = 3.0;
    let total_w = BAR_COUNT as f32 * (width + gap);
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(total_w, height), egui::Sense::hover());
    let painter = ui.painter();
    for (i, &level) in bars.iter().enumerate() {
        let bar_h = height * level;
        let x = rect.left() + i as f32 * (width + gap);
        let y = rect.bottom() - bar_h;
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, bar_h)),
            2.0,
            egui::Color32::from_rgb(220, 80, 80),
        );
    }
}

fn truncate_middle(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(1) / 2;
    let start: String = text.chars().take(keep).collect();
    let end: String = text
        .chars()
        .rev()
        .take(keep)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{start}…{end}")
}

fn position_top_center(ctx: &egui::Context) {
    if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
        let x = ((monitor.x - OVERLAY_WIDTH) * 0.5).max(0.0);
        let y = 28.0;
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
    } else {
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(200.0, 28.0)));
    }
}
