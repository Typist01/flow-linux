pub mod catalogs;

use catalogs::{
    LOCAL_WHISPER_MODELS, OPENAI_POLISH_MODELS, OPENAI_STT_MODELS, OPENAI_STREAMING_STT_MODELS,
    STREAMING_DELAYS,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application ID for KDE portal / global shortcuts.
pub const APP_ID: &str = "io.flowlinux.SpikeHotkey";
pub const SHORTCUT_ID: &str = "flow-dictation";
pub const DEFAULT_HOTKEY: &str = "Meta+Ctrl+Space";

pub const SAMPLE_RATE: u32 = 16_000;
pub const STREAMING_SAMPLE_RATE: u32 = 24_000;
pub const SILENCE_RMS_THRESHOLD: f32 = 0.005;
pub const PRE_INJECT_DELAY_MS: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SttProvider {
    #[default]
    Local,
    Openai,
    Deepgram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SttMode {
    #[default]
    Batch,
    Streaming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PolishProvider {
    #[default]
    None,
    Ollama,
    Openai,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub show_overlay: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            autostart: true,
            show_overlay: true,
            log_level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    #[serde(default = "default_app_id")]
    pub app_id: String,
    #[serde(default = "default_shortcut_id")]
    pub shortcut_id: String,
    #[serde(default = "default_hotkey")]
    pub hotkey_trigger: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            app_id: default_app_id(),
            shortcut_id: default_shortcut_id(),
            hotkey_trigger: default_hotkey(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    #[serde(default)]
    pub provider: SttProvider,
    #[serde(default)]
    pub mode: SttMode,
    #[serde(default = "default_model_name")]
    pub model: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_openai_stt_model")]
    pub openai_model: String,
    #[serde(default = "default_streaming_model")]
    pub streaming_model: String,
    #[serde(default = "default_streaming_delay")]
    pub streaming_delay: String,
    #[serde(default = "default_openai_api_key_env")]
    pub openai_api_key_env: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider: SttProvider::Local,
            mode: SttMode::Batch,
            model: default_model_name(),
            language: default_language(),
            openai_model: default_openai_stt_model(),
            streaming_model: default_streaming_model(),
            streaming_delay: default_streaming_delay(),
            openai_api_key_env: default_openai_api_key_env(),
        }
    }
}

impl SttConfig {
    pub fn is_streaming(&self) -> bool {
        self.mode == SttMode::Streaming
    }

    pub fn active_model(&self) -> &str {
        if self.is_streaming() {
            return &self.streaming_model;
        }
        match self.provider {
            SttProvider::Local => &self.model,
            SttProvider::Openai => &self.openai_model,
            SttProvider::Deepgram => &self.openai_model,
        }
    }

    pub fn available_models(&self) -> &'static [&'static str] {
        if self.is_streaming() {
            return OPENAI_STREAMING_STT_MODELS;
        }
        match self.provider {
            SttProvider::Local => LOCAL_WHISPER_MODELS,
            SttProvider::Openai => OPENAI_STT_MODELS,
            SttProvider::Deepgram => &[],
        }
    }

    pub fn available_streaming_delays(&self) -> &'static [&'static str] {
        STREAMING_DELAYS
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider: PolishProvider,
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default = "default_openai_polish_model")]
    pub openai_model: String,
    #[serde(default = "default_openai_api_key_env")]
    pub openai_api_key_env: String,
}

impl Default for PolishConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: PolishProvider::None,
            ollama_url: default_ollama_url(),
            ollama_model: default_ollama_model(),
            openai_model: default_openai_polish_model(),
            openai_api_key_env: default_openai_api_key_env(),
        }
    }
}

impl PolishConfig {
    pub fn active_model(&self) -> &str {
        match self.provider {
            PolishProvider::None => "",
            PolishProvider::Ollama => &self.ollama_model,
            PolishProvider::Openai => &self.openai_model,
        }
    }

    pub fn available_models(&self) -> &'static [&'static str] {
        match self.provider {
            PolishProvider::None => &[],
            PolishProvider::Ollama => &[],
            PolishProvider::Openai => OPENAI_POLISH_MODELS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectConfig {
    #[serde(default = "default_true")]
    pub via_paste: bool,
}

impl Default for InjectConfig {
    fn default() -> Self {
        Self { via_paste: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub stt: SttConfig,
    #[serde(default)]
    pub polish: PolishConfig,
    #[serde(default)]
    pub inject: InjectConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            hotkey: HotkeyConfig::default(),
            stt: SttConfig::default(),
            polish: PolishConfig::default(),
            inject: InjectConfig::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("flow-linux/config.toml")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("flow-linux")
    }

    pub fn lock_path() -> PathBuf {
        Self::data_dir().join("flow-daemon.lock")
    }

    pub fn model_path(&self) -> PathBuf {
        let cache = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        cache
            .join("flow-linux/models")
            .join(format!("ggml-{}.bin", self.stt.model))
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match Self::load_from(&path) {
            Ok(config) => {
                tracing::info!(path = %path.display(), "loaded config");
                config
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "using defaults");
                let config = Self::default();
                if let Err(write_err) = config.save() {
                    tracing::warn!(error = %write_err, "could not write default config");
                }
                config
            }
        }
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let mut config: Config = toml::from_str(&contents)?;
        config.apply_env_overrides();
        config.normalize();
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let mut config = self.clone();
        config.normalize();
        let contents = toml::to_string_pretty(&config)?;
        fs::write(&path, contents).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })?;
        tracing::info!(path = %path.display(), "saved config");
        Ok(())
    }

    fn normalize(&mut self) {
        if self.stt.mode == SttMode::Streaming {
            self.stt.provider = SttProvider::Openai;
            if !OPENAI_STREAMING_STT_MODELS.contains(&self.stt.streaming_model.as_str()) {
                self.stt.streaming_model = default_streaming_model();
            }
            if !STREAMING_DELAYS.contains(&self.stt.streaming_delay.as_str()) {
                self.stt.streaming_delay = default_streaming_delay();
            }
        } else if !self.stt.available_models().contains(&self.stt.active_model()) {
            if let Some(first) = self.stt.available_models().first() {
                match self.stt.provider {
                    SttProvider::Local => self.stt.model = (*first).to_string(),
                    SttProvider::Openai | SttProvider::Deepgram => {
                        self.stt.openai_model = (*first).to_string();
                    }
                }
            }
        }

        if self.polish.enabled && self.polish.provider == PolishProvider::None {
            self.polish.provider = PolishProvider::Openai;
        }

        if !self.polish.enabled {
            self.polish.provider = PolishProvider::None;
        }

        if self.polish.enabled
            && !self
                .polish
                .available_models()
                .contains(&self.polish.active_model())
        {
            if let Some(first) = self.polish.available_models().first() {
                self.polish.openai_model = (*first).to_string();
            }
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(hotkey) = std::env::var("FLOW_HOTKEY") {
            self.hotkey.hotkey_trigger = hotkey;
        }
    }

    pub fn desktop_file_name(&self) -> String {
        format!("{}.desktop", self.hotkey.app_id)
    }

    pub fn app_id(&self) -> &str {
        &self.hotkey.app_id
    }

    pub fn shortcut_id(&self) -> &str {
        &self.hotkey.shortcut_id
    }

    pub fn hotkey_trigger(&self) -> &str {
        &self.hotkey.hotkey_trigger
    }

    pub fn language(&self) -> &str {
        &self.stt.language
    }

    pub fn inject_via_paste(&self) -> bool {
        self.inject.via_paste
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config at {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write config at {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config TOML")]
    Toml(#[from] toml::de::Error),
    #[error("failed to serialize config")]
    TomlSerialize(#[from] toml::ser::Error),
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_app_id() -> String {
    APP_ID.to_string()
}

fn default_shortcut_id() -> String {
    SHORTCUT_ID.to_string()
}

fn default_hotkey() -> String {
    DEFAULT_HOTKEY.to_string()
}

fn default_model_name() -> String {
    "base.en".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_openai_stt_model() -> String {
    "gpt-4o-mini-transcribe".to_string()
}

fn default_streaming_model() -> String {
    "gpt-realtime-whisper".to_string()
}

fn default_streaming_delay() -> String {
    "low".to_string()
}

fn default_openai_api_key_env() -> String {
    "OPENAI_API_KEY".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama3.2:3b".to_string()
}

fn default_openai_polish_model() -> String {
    "gpt-4o-mini".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_batch() {
        let config = Config::default();
        assert_eq!(config.stt.mode, SttMode::Batch);
        assert!(!config.stt.is_streaming());
    }

    #[test]
    fn streaming_normalize_forces_openai_and_valid_model() {
        let mut config = Config::default();
        config.stt.mode = SttMode::Streaming;
        config.stt.provider = SttProvider::Local;
        config.stt.streaming_model = "not-a-model".into();
        config.stt.streaming_delay = "nope".into();
        config.normalize();
        assert_eq!(config.stt.provider, SttProvider::Openai);
        assert_eq!(config.stt.streaming_model, "gpt-realtime-whisper");
        assert_eq!(config.stt.streaming_delay, "low");
    }

    #[test]
    fn missing_mode_deserializes_as_batch() {
        let toml = r#"
[stt]
provider = "openai"
openai_model = "gpt-4o-mini-transcribe"
"#;
        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.stt.mode, SttMode::Batch);
        assert_eq!(config.stt.streaming_model, "gpt-realtime-whisper");
    }

    #[test]
    fn show_overlay_defaults_true() {
        let config = Config::default();
        assert!(config.general.show_overlay);

        let toml = r#"
[general]
autostart = true
"#;
        let config: Config = toml::from_str(toml).expect("parse");
        assert!(config.general.show_overlay);
    }
}
