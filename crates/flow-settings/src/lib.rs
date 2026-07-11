use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use flow_config::catalogs::{polish_provider_label, stt_provider_label};
use flow_config::{Config, PolishProvider, SttProvider};
use flow_secrets::{
    has_openai_api_key, store_openai_api_key, validate_configured_openai_api_key,
    validate_openai_api_key,
};
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};

static OPEN_TX: OnceLock<Sender<()>> = OnceLock::new();

/// Long-lived settings thread. Must be called once at daemon startup.
pub fn start_settings_service(reload_tx: Sender<()>) -> JoinHandle<()> {
    let (open_tx, open_rx) = crossbeam_channel::unbounded();
    let _ = OPEN_TX.set(open_tx);

    thread::Builder::new()
        .name("flow-settings".into())
        .spawn(move || settings_service_loop(open_rx, reload_tx))
        .expect("failed to spawn settings service")
}

/// Request the settings window. Safe to call repeatedly from the tray.
pub fn open_settings_window() {
    match OPEN_TX.get() {
        Some(tx) => {
            if tx.send(()).is_err() {
                tracing::error!("settings service is not running");
            }
        }
        None => tracing::error!("settings service not started"),
    }
}

fn settings_service_loop(open_rx: Receiver<()>, reload_tx: Sender<()>) {
    while open_rx.recv().is_ok() {
        let reload_tx = reload_tx.clone();
        let options = build_native_options();

        if let Err(error) = eframe::run_native(
            "Flow Linux Settings",
            options,
            Box::new(move |_cc| Ok(Box::new(SettingsApp::new(reload_tx)))),
        ) {
            tracing::error!(%error, "settings window failed");
        }
    }
}

fn build_native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 620.0])
            .with_min_inner_size([420.0, 480.0]),
        centered: true,
        run_and_return: true,
        event_loop_builder: Some(Box::new(|builder| {
            #[cfg(target_os = "linux")]
            {
                use winit::platform::wayland::EventLoopBuilderExtWayland;
                use winit::platform::x11::EventLoopBuilderExtX11;
                EventLoopBuilderExtWayland::with_any_thread(builder, true);
                EventLoopBuilderExtX11::with_any_thread(builder, true);
            }
            #[cfg(target_os = "windows")]
            {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                EventLoopBuilderExtWindows::with_any_thread(builder, true);
            }
        })),
        ..Default::default()
    }
}

struct SettingsApp {
    config: Config,
    openai_api_key: String,
    status_message: Option<(String, bool)>,
    reload_tx: Sender<()>,
}

impl SettingsApp {
    fn new(reload_tx: Sender<()>) -> Self {
        let config = Config::load();
        let openai_api_key = String::new();
        Self {
            config,
            openai_api_key,
            status_message: None,
            reload_tx,
        }
    }

    fn set_status(&mut self, message: impl Into<String>, success: bool) {
        self.status_message = Some((message.into(), success));
    }

    fn save(&mut self) {
        let has_stored_key = has_openai_api_key();
        let has_typed_key = !self.openai_api_key.trim().is_empty();
        let has_env_key = std::env::var("OPENAI_API_KEY")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        if self.config.stt.provider == SttProvider::Openai
            && !has_typed_key
            && !has_stored_key
            && !has_env_key
        {
            self.set_status("OpenAI API key is required for cloud STT", false);
            return;
        }

        if self.config.polish.enabled
            && self.config.polish.provider == PolishProvider::Openai
            && !has_typed_key
            && !has_stored_key
            && !has_env_key
        {
            self.set_status("OpenAI API key is required for cloud polish", false);
            return;
        }

        if has_typed_key {
            match store_openai_api_key(&self.openai_api_key) {
                Ok(()) => {}
                Err(e) => {
                    self.set_status(format!("Failed to store API key: {e}"), false);
                    return;
                }
            }
        }

        match self.config.save() {
            Ok(()) => {
                let _ = self.reload_tx.send(());
                if has_typed_key {
                    self.openai_api_key.clear();
                }
                self.set_status("Saved — changes apply to the next dictation", true);
            }
            Err(e) => self.set_status(format!("Failed to save config: {e}"), false),
        }
    }

    fn validate_key(&mut self) {
        let result = if !self.openai_api_key.trim().is_empty() {
            validate_openai_api_key(&self.openai_api_key)
        } else {
            validate_configured_openai_api_key(&self.config.stt.openai_api_key_env)
        };

        match result {
            Ok(()) => self.set_status("API key is valid", true),
            Err(e) => self.set_status(format!("API key validation failed: {e}"), false),
        }
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Flow Linux Settings");
            ui.label("Configure speech-to-text, polish, and cloud providers.");
            ui.separator();

            ui.collapsing("Speech-to-Text", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Provider");
                    egui::ComboBox::from_id_salt("stt_provider")
                        .selected_text(stt_provider_label(self.config.stt.provider))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.config.stt.provider,
                                SttProvider::Local,
                                stt_provider_label(SttProvider::Local),
                            );
                            ui.selectable_value(
                                &mut self.config.stt.provider,
                                SttProvider::Openai,
                                stt_provider_label(SttProvider::Openai),
                            );
                            ui.add_enabled_ui(false, |ui| {
                                ui.selectable_value(
                                    &mut self.config.stt.provider,
                                    SttProvider::Deepgram,
                                    stt_provider_label(SttProvider::Deepgram),
                                );
                            });
                        });
                });

                let models = self.config.stt.available_models().to_vec();
                if !models.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Model");
                        let active = self.config.stt.active_model().to_string();
                        let mut selected = active.clone();
                        egui::ComboBox::from_id_salt("stt_model")
                            .selected_text(&active)
                            .show_ui(ui, |ui| {
                                for model in models {
                                    ui.selectable_value(&mut selected, model.to_string(), model);
                                }
                            });
                        match self.config.stt.provider {
                            SttProvider::Local => self.config.stt.model = selected,
                            SttProvider::Openai | SttProvider::Deepgram => {
                                self.config.stt.openai_model = selected
                            }
                        }
                    });
                }

                ui.horizontal(|ui| {
                    ui.label("Language");
                    ui.text_edit_singleline(&mut self.config.stt.language);
                });

                if self.config.stt.provider == SttProvider::Local {
                    ui.label(format!(
                        "Local model path: {}",
                        self.config.model_path().display()
                    ));
                }
            });

            ui.add_space(8.0);

            ui.collapsing("Polish", |ui| {
                ui.checkbox(&mut self.config.polish.enabled, "Enable polish after transcription");

                if self.config.polish.enabled {
                    ui.horizontal(|ui| {
                        ui.label("Provider");
                        egui::ComboBox::from_id_salt("polish_provider")
                            .selected_text(polish_provider_label(self.config.polish.provider))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.config.polish.provider,
                                    PolishProvider::Openai,
                                    polish_provider_label(PolishProvider::Openai),
                                );
                                ui.add_enabled_ui(false, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.polish.provider,
                                        PolishProvider::Ollama,
                                        polish_provider_label(PolishProvider::Ollama),
                                    );
                                });
                            });
                    });

                    let models = self.config.polish.available_models().to_vec();
                    if !models.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Model");
                            let active = self.config.polish.active_model().to_string();
                            let mut selected = active.clone();
                            egui::ComboBox::from_id_salt("polish_model")
                                .selected_text(&active)
                                .show_ui(ui, |ui| {
                                    for model in models {
                                        ui.selectable_value(
                                            &mut selected,
                                            model.to_string(),
                                            model,
                                        );
                                    }
                                });
                            self.config.polish.openai_model = selected;
                        });
                    }
                } else {
                    self.config.polish.provider = PolishProvider::None;
                }
            });

            ui.add_space(8.0);

            ui.collapsing("API Keys", |ui| {
                ui.label("OpenAI API key (stored in KDE Wallet / system keyring)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.openai_api_key)
                        .password(true)
                        .desired_width(f32::INFINITY)
                        .hint_text("sk-..."),
                );
                ui.label("Leave blank on Save to keep the existing stored key.");
                if has_openai_api_key() {
                    ui.label("A key is configured (keyring or OPENAI_API_KEY).");
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 90, 90),
                        "No API key configured yet.",
                    );
                }
                ui.label("Environment fallback: OPENAI_API_KEY");
            });

            ui.add_space(8.0);

            ui.collapsing("Injection", |ui| {
                ui.checkbox(
                    &mut self.config.inject.via_paste,
                    "Use clipboard paste (recommended on KDE)",
                );
            });

            ui.add_space(12.0);

            if let Some((message, success)) = &self.status_message {
                let color = if *success {
                    egui::Color32::from_rgb(80, 170, 90)
                } else {
                    egui::Color32::from_rgb(220, 90, 90)
                };
                ui.colored_label(color, message);
            }

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    self.save();
                }
                if ui.button("Validate API key").clicked() {
                    self.validate_key();
                }
                if ui.button("Reload from disk").clicked() {
                    self.config = Config::load();
                    self.openai_api_key.clear();
                    self.set_status("Reloaded config from disk", true);
                }
            });
        });
    }
}
