mod noop;
mod ollama;
mod openai;

use flow_config::{Config, PolishProvider};
use std::sync::Arc;
use thiserror::Error;

pub use noop::NoOpPolishEngine;
pub use openai::OpenAiPolishEngine;

/// Text cleanup after STT. Cloud/local LLM backends implement this trait.
#[async_trait::async_trait]
pub trait PolishEngine: Send + Sync {
    fn provider(&self) -> PolishProvider;
    async fn polish(&self, text: &str) -> Result<String, PolishError>;
}

#[derive(Debug, Error)]
pub enum PolishError {
    #[error("provider {0:?} is not implemented yet")]
    NotImplemented(PolishProvider),
    #[error("polish request failed: {0}")]
    Request(String),
}

pub fn build_polish_engine(config: &Config) -> Result<Arc<dyn PolishEngine>, PolishError> {
    if !config.polish.enabled || config.polish.provider == PolishProvider::None {
        return Ok(Arc::new(NoOpPolishEngine));
    }

    match config.polish.provider {
        PolishProvider::None => Ok(Arc::new(NoOpPolishEngine)),
        PolishProvider::Openai => Ok(Arc::new(OpenAiPolishEngine::new(config)?)),
        PolishProvider::Ollama => Ok(Arc::new(ollama::OllamaPolishEngine)),
    }
}
