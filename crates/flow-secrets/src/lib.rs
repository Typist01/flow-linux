use thiserror::Error;

pub const OPENAI_KEY_ACCOUNT: &str = "openai-api-key";

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("API key not found — set it in Settings or export OPENAI_API_KEY")]
    MissingApiKey,
    #[error("stored key could not be read back from keyring")]
    ReadBackFailed,
}

pub fn store_openai_api_key(key: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new("flow-linux", OPENAI_KEY_ACCOUNT)?;
    if key.trim().is_empty() {
        let _ = entry.delete_credential();
        Ok(())
    } else {
        entry.set_password(key.trim())?;
        let read_back = entry.get_password()?;
        if read_back.trim() != key.trim() {
            return Err(SecretError::ReadBackFailed);
        }
        Ok(())
    }
}

pub fn get_openai_api_key() -> Result<Option<String>, SecretError> {
    let entry = keyring::Entry::new("flow-linux", OPENAI_KEY_ACCOUNT)?;
    match entry.get_password() {
        Ok(key) if !key.trim().is_empty() => Ok(Some(key)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn has_openai_api_key() -> bool {
    get_openai_api_key()
        .ok()
        .flatten()
        .is_some()
        || std::env::var("OPENAI_API_KEY")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

/// Resolve OpenAI API key: keyring first, then env var fallback.
pub fn resolve_openai_api_key(env_var: &str) -> Result<String, SecretError> {
    if let Some(key) = get_openai_api_key()? {
        return Ok(key);
    }
    if let Ok(key) = std::env::var(env_var) {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    Err(SecretError::MissingApiKey)
}

/// Ping OpenAI with the given key (lightweight auth check).
pub fn validate_openai_api_key(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key is empty".to_string());
    }

    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key)
        .send()
        .map_err(|e| format!("network error: {e}"))?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().unwrap_or_default();
    Err(format!("OpenAI rejected key ({status}): {body}"))
}

/// Validate whichever key is currently configured (typed, stored, or env).
pub fn validate_configured_openai_api_key(env_var: &str) -> Result<(), String> {
    if let Ok(key) = resolve_openai_api_key(env_var) {
        return validate_openai_api_key(&key);
    }
    Err("No API key configured".to_string())
}
