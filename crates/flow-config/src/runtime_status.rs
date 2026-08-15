use crate::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeStatus {
    pub mic_device: Option<String>,
    pub hotkey_bound: bool,
    pub hotkey_trigger: String,
    pub updated_unix: u64,
}

impl RuntimeStatus {
    pub fn path() -> PathBuf {
        Config::data_dir().join("runtime-status.toml")
    }

    pub fn write(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let contents = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, contents).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load() -> Option<Self> {
        let contents = fs::read_to_string(Self::path()).ok()?;
        toml::from_str(&contents).ok()
    }

    pub fn now(mic_device: Option<String>, hotkey_bound: bool, hotkey_trigger: String) -> Self {
        let updated_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            mic_device,
            hotkey_bound,
            hotkey_trigger,
            updated_unix,
        }
    }
}
