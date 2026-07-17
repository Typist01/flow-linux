use crossbeam_channel::Sender;
use eframe::egui;
use flow_config::catalogs::{polish_provider_label, stt_mode_label, stt_provider_label};
use flow_config::{Config, PolishProvider, SttMode, SttProvider};
use flow_secrets::{
    has_openai_api_key, store_openai_api_key, validate_configured_openai_api_key,
    validate_openai_api_key,
};
use std::sync::OnceLock;

static OPEN_TX: OnceLock<Sender<()>> = OnceLock::new();

/// Register the UI-thread channel used by [`open_settings_window`].
/// Called once from the overlay/UI event loop owner at startup.
pub fn register_settings_opener(open_tx: Sender<()>) {
    if OPEN_TX.set(open_tx).is_err() {
        tracing::warn!("settings opener already registered");
    }
}

/// Request the settings window. Safe to call repeatedly from the tray/daemon.
pub fn open_settings_window() {
    match OPEN_TX.get() {
        Some(tx) => {
            if tx.send(()).is_err() {
                tracing::error!("settings UI thread is not running");
            }
        }
        None => tracing::error!("settings opener not registered"),
    }
}

pub struct SettingsApp {
    config: Config,
    openai_api_key: String,
    status_message: Option<(String, bool)>,
    reload_tx: Sender<()>,
}

impl SettingsApp {
    pub fn new(reload_tx: Sender<()>) -> Self {
        Self {
            config: Config::load(),
            openai_api_key: String::new(),
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

        if (self.config.stt.provider == SttProvider::Openai
            || self.config.stt.mode == SttMode::Streaming)
            && !has_typed_key
            && !has_stored_key
            && !has_env_key
        {
            self.set_status("OpenAI API key is required for cloud / streaming STT", false);
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

    /// Draw the settings form into an egui UI (root window or secondary viewport).
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Flow Linux Settings");
        ui.label("Configure speech-to-text, polish, and cloud providers.");
        ui.separator();

        ui.collapsing("General", |ui| {
            ui.checkbox(
                &mut self.config.general.show_overlay,
                "Show listening overlay",
            );
            ui.label("Live partial text (streaming) and waveform while the hotkey is held.");
        });

        ui.add_space(8.0);

        ui.collapsing("Speech-to-Text", |ui| {
            ui.horizontal(|ui| {
                ui.label("Mode");
                egui::ComboBox::from_id_salt("stt_mode")
                    .selected_text(stt_mode_label(self.config.stt.mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.config.stt.mode,
                            SttMode::Batch,
                            stt_mode_label(SttMode::Batch),
                        );
                        ui.selectable_value(
                            &mut self.config.stt.mode,
                            SttMode::Streaming,
                            stt_mode_label(SttMode::Streaming),
                        );
                    });
            });

            if self.config.stt.mode == SttMode::Streaming {
                self.config.stt.provider = SttProvider::Openai;
                ui.label("Streaming uses OpenAI Realtime (gpt-realtime-whisper).");

                let models = self.config.stt.available_models().to_vec();
                ui.horizontal(|ui| {
                    ui.label("Model");
                    let active = self.config.stt.streaming_model.clone();
                    let mut selected = active.clone();
                    egui::ComboBox::from_id_salt("streaming_model")
                        .selected_text(&active)
                        .show_ui(ui, |ui| {
                            for model in models {
                                ui.selectable_value(&mut selected, model.to_string(), model);
                            }
                        });
                    self.config.stt.streaming_model = selected;
                });

                let delays = self.config.stt.available_streaming_delays().to_vec();
                ui.horizontal(|ui| {
                    ui.label("Delay");
                    let active = self.config.stt.streaming_delay.clone();
                    let mut selected = active.clone();
                    egui::ComboBox::from_id_salt("streaming_delay")
                        .selected_text(&active)
                        .show_ui(ui, |ui| {
                            for delay in delays {
                                ui.selectable_value(&mut selected, delay.to_string(), delay);
                            }
                        });
                    self.config.stt.streaming_delay = selected;
                });
            } else {
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

                if self.config.stt.provider == SttProvider::Local {
                    ui.label(format!(
                        "Local model path: {}",
                        self.config.model_path().display()
                    ));
                }
            }

            ui.horizontal(|ui| {
                ui.label("Language");
                ui.text_edit_singleline(&mut self.config.stt.language);
            });
        });

        ui.add_space(8.0);

        ui.collapsing("Polish", |ui| {
            ui.checkbox(
                &mut self.config.polish.enabled,
                "Enable polish after transcription",
            );

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
                                    ui.selectable_value(&mut selected, model.to_string(), model);
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
    }
}
