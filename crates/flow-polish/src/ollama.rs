use crate::{PolishEngine, PolishError};
use flow_config::PolishProvider;

pub struct OllamaPolishEngine;

#[async_trait::async_trait]
impl PolishEngine for OllamaPolishEngine {
    fn provider(&self) -> PolishProvider {
        PolishProvider::Ollama
    }

    async fn polish(&self, _text: &str) -> Result<String, PolishError> {
        Err(PolishError::NotImplemented(PolishProvider::Ollama))
    }
}
