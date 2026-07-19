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

const TEAL: egui::Color32 = egui::Color32::from_rgb(0x2B, 0xB8, 0xA8);
const EMBER: egui::Color32 = egui::Color32::from_rgb(0xE4, 0x57, 0x4D);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0x9A, 0x97, 0x90);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x1D, 0x24);
const CHIP_BG: egui::Color32 = egui::Color32::from_rgb(0x24, 0x28, 0x30);

/// Register the UI-thread channel used by [`open_settings_window`].
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsPage {
    Ready,
    Dictation,
    Voice,
    Polish,
    Injection,
    About,
}

pub struct SettingsApp {
    config: Config,
    openai_api_key: String,
    status_message: Option<(String, bool)>,
    reload_tx: Sender<()>,
    page: SettingsPage,
}

impl SettingsApp {
    pub fn new(reload_tx: Sender<()>) -> Self {
        Self {
            config: Config::load(),
            openai_api_key: String::new(),
            status_message: None,
            reload_tx,
            page: SettingsPage::Ready,
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
            self.set_status(
                "OpenAI API key is required for cloud / streaming STT",
                false,
            );
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
            validate_openai_api_key(&self.openai_api_key, &self.config.stt.openai_api_base)
        } else {
            validate_configured_openai_api_key(
                &self.config.stt.openai_api_key_env,
                &self.config.stt.openai_api_base,
            )
        };

        match result {
            Ok(()) => self.set_status("API key is valid", true),
            Err(e) => self.set_status(format!("API key validation failed: {e}"), false),
        }
    }

    /// Draw the settings instrument panel.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.ready_card(ui);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            self.nav(ui);
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings_page")
                    .show(ui, |ui| match self.page {
                        SettingsPage::Ready => self.page_ready(ui),
                        SettingsPage::Dictation => self.page_dictation(ui),
                        SettingsPage::Voice => self.page_voice(ui),
                        SettingsPage::Polish => self.page_polish(ui),
                        SettingsPage::Injection => self.page_injection(ui),
                        SettingsPage::About => self.page_about(ui),
                    });
            });
        });

        ui.add_space(12.0);
        if let Some((message, success)) = &self.status_message {
            let color = if *success { TEAL } else { EMBER };
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

    fn ready_card(&self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(SURFACE)
            .corner_radius(12.0)
            .inner_margin(egui::Margin::symmetric(14, 12))
            .stroke(egui::Stroke::new(1.0_f32, CHIP_BG))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Ready to dictate").strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ready = self.is_ready();
                        chip(
                            ui,
                            if ready { "Ready" } else { "Needs setup" },
                            if ready { TEAL } else { EMBER },
                        );
                    });
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    chip(
                        ui,
                        if self.config.stt.is_streaming() {
                            "Streaming"
                        } else {
                            "Batch"
                        },
                        TEAL,
                    );
                    chip(
                        ui,
                        if has_openai_api_key() {
                            "API key: Configured"
                        } else {
                            "API key: Missing"
                        },
                        if has_openai_api_key() { TEAL } else { EMBER },
                    );
                    chip(
                        ui,
                        if self.config.general.show_overlay {
                            "Overlay on"
                        } else {
                            "Overlay off"
                        },
                        MUTED,
                    );
                    chip(
                        ui,
                        if self.config.polish.enabled {
                            "Polish on"
                        } else {
                            "Polish off"
                        },
                        MUTED,
                    );
                    chip(
                        ui,
                        &format!("Hotkey {}", self.config.hotkey.hotkey_trigger),
                        MUTED,
                    );
                });
            });
    }

    fn is_ready(&self) -> bool {
        match self.config.stt.mode {
            SttMode::Streaming => has_openai_api_key(),
            SttMode::Batch => match self.config.stt.provider {
                SttProvider::Openai => has_openai_api_key(),
                SttProvider::Local => self.config.model_path().exists(),
                SttProvider::Deepgram => false,
            },
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_width(120.0);
            for (page, label) in [
                (SettingsPage::Ready, "Ready"),
                (SettingsPage::Dictation, "Dictation"),
                (SettingsPage::Voice, "Voice"),
                (SettingsPage::Polish, "Polish"),
                (SettingsPage::Injection, "Injection"),
                (SettingsPage::About, "About"),
            ] {
                let selected = self.page == page;
                let text = if selected {
                    egui::RichText::new(label).strong().color(TEAL)
                } else {
                    egui::RichText::new(label)
                };
                if ui.add(egui::SelectableLabel::new(selected, text)).clicked() {
                    self.page = page;
                }
            }
        });
    }

    fn page_ready(&mut self, ui: &mut egui::Ui) {
        ui.heading("Status");
        ui.label("Am I ready to dictate? Fix anything red, then Save.");
        ui.add_space(8.0);

        status_row(
            ui,
            "Speech mode",
            if self.config.stt.is_streaming() {
                "Streaming (live)"
            } else {
                "Batch (upload on release)"
            },
            true,
        );
        status_row(
            ui,
            "OpenAI API key",
            if has_openai_api_key() {
                "Configured (keyring or env)"
            } else {
                "Missing — set under Voice"
            },
            has_openai_api_key()
                || matches!(
                    (self.config.stt.mode, self.config.stt.provider),
                    (SttMode::Batch, SttProvider::Local)
                ),
        );
        if self.config.stt.mode == SttMode::Batch && self.config.stt.provider == SttProvider::Local
        {
            let ok = self.config.model_path().exists();
            status_row(
                ui,
                "Local Whisper model",
                if ok {
                    "Found on disk"
                } else {
                    "Missing — see Voice page"
                },
                ok,
            );
        }
        status_row(
            ui,
            "Listening overlay",
            if self.config.general.show_overlay {
                "Enabled"
            } else {
                "Disabled"
            },
            true,
        );
        status_row(
            ui,
            "Injection",
            if self.config.inject.via_paste {
                "Clipboard paste"
            } else {
                "ydotool type"
            },
            true,
        );

        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("API keys stay on your machine (system keyring or OPENAI_API_KEY). Nothing is shipped in the AppImage.")
                .color(MUTED)
                .small(),
        );
    }

    fn page_dictation(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dictation");
        ui.label("How audio becomes text.");
        ui.add_space(8.0);

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

        ui.add_space(8.0);
        ui.checkbox(
            &mut self.config.general.show_overlay,
            "Show listening overlay (Flow Capsule)",
        );
    }

    fn page_voice(&mut self, ui: &mut egui::Ui) {
        ui.heading("Voice / API");
        ui.label("Bring your own key. Never stored in the AppImage.");
        ui.add_space(8.0);

        ui.label("OpenAI API key (stored in KDE Wallet / system keyring)");
        ui.add(
            egui::TextEdit::singleline(&mut self.openai_api_key)
                .password(true)
                .desired_width(f32::INFINITY)
                .hint_text("sk-…"),
        );
        ui.label("Leave blank on Save to keep the existing stored key.");
        if has_openai_api_key() {
            chip(ui, "Configured", TEAL);
        } else {
            chip(ui, "Missing", EMBER);
        }
        ui.label(
            egui::RichText::new("Environment fallback: OPENAI_API_KEY")
                .small()
                .color(MUTED),
        );

        ui.add_space(12.0);
        ui.label(egui::RichText::new("Hotkey").strong());
        ui.label(
            egui::RichText::new(&self.config.hotkey.hotkey_trigger)
                .monospace()
                .color(TEAL),
        );
        ui.label(
            egui::RichText::new("Change via KDE System Settings → Shortcuts (portal).")
                .small()
                .color(MUTED),
        );
    }

    fn page_polish(&mut self, ui: &mut egui::Ui) {
        ui.heading("Polish");
        ui.label("Optional cleanup after the final transcript (never on live partials).");
        ui.add_space(8.0);

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
    }

    fn page_injection(&mut self, ui: &mut egui::Ui) {
        ui.heading("Injection");
        ui.label("How text lands in the focused app.");
        ui.add_space(8.0);
        ui.checkbox(
            &mut self.config.inject.via_paste,
            "Use clipboard paste (recommended on KDE)",
        );
        ui.label(
            egui::RichText::new("Requires wl-clipboard and ydotool user service.")
                .small()
                .color(MUTED),
        );
    }

    fn page_about(&mut self, ui: &mut egui::Ui) {
        ui.heading("Flow Linux");
        ui.label("Signal identity — voice as a flowing current that settles into text.");
        ui.add_space(8.0);
        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
        ui.label("License: see repository LICENSE / OFL fonts in assets/ATTRIBUTIONS.md");
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Bring your own key (BYOK)").strong());
        ui.label("No API keys are bundled. Keys live in your system keyring or OPENAI_API_KEY.");
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Host deps for distribution: PipeWire, wl-clipboard, ydotool, libsecret, desktop portal.")
                .small()
                .color(MUTED),
        );
    }
}

fn chip(ui: &mut egui::Ui, text: &str, accent: egui::Color32) {
    egui::Frame::NONE
        .fill(CHIP_BG)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(accent).monospace());
        });
}

fn status_row(ui: &mut egui::Ui, label: &str, value: &str, ok: bool) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .color(if ok { TEAL } else { EMBER })
                    .strong(),
            );
        });
    });
    ui.add_space(4.0);
}
