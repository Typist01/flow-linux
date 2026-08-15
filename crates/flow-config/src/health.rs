use crate::runtime_status::RuntimeStatus;
use crate::{Config, SttMode, SttProvider};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Ok,
    Fail,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub id: &'static str,
    pub label: &'static str,
    pub state: CheckState,
    pub detail: String,
    pub fix: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub checks: Vec<HealthCheck>,
}

impl HealthSnapshot {
    pub fn collect(config: &Config, openai_key_present: bool) -> Self {
        let runtime = RuntimeStatus::load();
        let (mic_name, mic_mute) = probe_microphone(runtime.as_ref());
        let hotkey = probe_hotkey(config, runtime.as_ref());
        let ydotool = probe_ydotool();
        let model = probe_local_model(config);
        let key = probe_api_key(config, openai_key_present);

        Self {
            checks: vec![mic_name, mic_mute, hotkey, ydotool, model, key],
        }
    }

    pub fn is_ready(&self) -> bool {
        self.checks.iter().all(|check| check.state != CheckState::Fail)
    }

    pub fn check(&self, id: &str) -> Option<&HealthCheck> {
        self.checks.iter().find(|check| check.id == id)
    }
}

fn probe_microphone(runtime: Option<&RuntimeStatus>) -> (HealthCheck, HealthCheck) {
    let pactl_name = command_stdout("pactl", &["get-default-source"]);
    let mute_out = command_stdout("pactl", &["get-source-mute", "@DEFAULT_SOURCE@"]);

    let name = pactl_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| runtime.and_then(|s| s.mic_device.clone()));

    let mic_name = match name {
        Some(device) => HealthCheck {
            id: "mic-name",
            label: "Microphone",
            state: CheckState::Ok,
            detail: device,
            fix: None,
        },
        None => HealthCheck {
            id: "mic-name",
            label: "Microphone",
            state: CheckState::Unknown,
            detail: "Could not read the default source (is PipeWire running?)".into(),
            fix: Some("Install PipeWire Pulse and check System Settings → Audio".into()),
        },
    };

    let mic_mute = match mute_out.as_deref().map(parse_pactl_mute) {
        Some(Some(true)) => HealthCheck {
            id: "mic-mute",
            label: "Microphone mute",
            state: CheckState::Fail,
            detail: "Muted".into(),
            fix: Some("Unmute the system microphone (hardware mic-mute key or Audio settings)".into()),
        },
        Some(Some(false)) => HealthCheck {
            id: "mic-mute",
            label: "Microphone mute",
            state: CheckState::Ok,
            detail: "Unmuted".into(),
            fix: None,
        },
        _ => HealthCheck {
            id: "mic-mute",
            label: "Microphone mute",
            state: CheckState::Unknown,
            detail: "Mute state unknown".into(),
            fix: None,
        },
    };

    (mic_name, mic_mute)
}

fn probe_hotkey(config: &Config, runtime: Option<&RuntimeStatus>) -> HealthCheck {
    if let Some(status) = runtime {
        if status.hotkey_bound {
            return HealthCheck {
                id: "hotkey",
                label: "Hotkey",
                state: CheckState::Ok,
                detail: status.hotkey_trigger.clone(),
                fix: None,
            };
        }
        return HealthCheck {
            id: "hotkey",
            label: "Hotkey",
            state: CheckState::Fail,
            detail: "Not bound".into(),
            fix: Some(format!(
                "Assign {} in the shortcut dialog, then hold it to dictate",
                config.hotkey_trigger()
            )),
        };
    }

    HealthCheck {
        id: "hotkey",
        label: "Hotkey",
        state: CheckState::Unknown,
        detail: format!("Preferred {}", config.hotkey_trigger()),
        fix: Some("Start the daemon once so it can register the portal shortcut".into()),
    }
}

fn probe_ydotool() -> HealthCheck {
    let on_path = command_success("sh", &["-c", "command -v ydotool"]);
    let active = command_stdout("systemctl", &["--user", "is-active", "ydotool.service"])
        .or_else(|| command_stdout("systemctl", &["--user", "is-active", "ydotoold.service"]));
    let running = active
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| s == "active");

    match (on_path, running) {
        (true, true) => HealthCheck {
            id: "ydotool",
            label: "Injection (ydotool)",
            state: CheckState::Ok,
            detail: "ydotool service is active".into(),
            fix: None,
        },
        (false, _) => HealthCheck {
            id: "ydotool",
            label: "Injection (ydotool)",
            state: CheckState::Fail,
            detail: "ydotool is not installed".into(),
            fix: Some("Install ydotool and enable the user service".into()),
        },
        (true, false) => HealthCheck {
            id: "ydotool",
            label: "Injection (ydotool)",
            state: CheckState::Fail,
            detail: "ydotool is installed but the user service is not active".into(),
            fix: Some("systemctl --user enable --now ydotool.service".into()),
        },
    }
}

fn probe_local_model(config: &Config) -> HealthCheck {
    let required = config.stt.mode == SttMode::Batch && config.stt.provider == SttProvider::Local;
    let exists = config.model_path().exists();
    if !required {
        return HealthCheck {
            id: "local-model",
            label: "Local Whisper model",
            state: CheckState::Ok,
            detail: "Not required for the current mode".into(),
            fix: None,
        };
    }
    if exists {
        HealthCheck {
            id: "local-model",
            label: "Local Whisper model",
            state: CheckState::Ok,
            detail: config.model_path().display().to_string(),
            fix: None,
        }
    } else {
        HealthCheck {
            id: "local-model",
            label: "Local Whisper model",
            state: CheckState::Fail,
            detail: "Missing on disk".into(),
            fix: Some("Download the model from the Ready or Dictation page".into()),
        }
    }
}

fn probe_api_key(config: &Config, present: bool) -> HealthCheck {
    let required = config.stt.mode == SttMode::Streaming
        || config.stt.provider == SttProvider::Openai
        || (config.polish.enabled && matches!(config.polish.provider, crate::PolishProvider::Openai));
    if !required {
        return HealthCheck {
            id: "api-key",
            label: "OpenAI API key",
            state: CheckState::Ok,
            detail: "Not required for local batch STT".into(),
            fix: None,
        };
    }
    if present {
        HealthCheck {
            id: "api-key",
            label: "OpenAI API key",
            state: CheckState::Ok,
            detail: "Configured (keyring or env)".into(),
            fix: None,
        }
    } else {
        HealthCheck {
            id: "api-key",
            label: "OpenAI API key",
            state: CheckState::Fail,
            detail: "Missing".into(),
            fix: Some("Set the key under Voice, or export OPENAI_API_KEY".into()),
        }
    }
}

pub fn parse_pactl_mute(output: &str) -> Option<bool> {
    for line in output.lines() {
        let line = line.trim().to_ascii_lowercase();
        if let Some(rest) = line.strip_prefix("mute:") {
            return match rest.trim() {
                "yes" | "true" | "1" => Some(true),
                "no" | "false" | "0" => Some(false),
                _ => None,
            };
        }
    }
    None
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn command_success(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::parse_pactl_mute;

    #[test]
    fn parses_pactl_mute_yes_no() {
        assert_eq!(parse_pactl_mute("Mute: yes\n"), Some(true));
        assert_eq!(parse_pactl_mute("Mute: no\n"), Some(false));
        assert_eq!(parse_pactl_mute("garbage"), None);
    }
}
