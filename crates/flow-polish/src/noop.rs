use crate::{PolishEngine, PolishError};
use flow_config::PolishProvider;

pub struct NoOpPolishEngine;

#[async_trait::async_trait]
impl PolishEngine for NoOpPolishEngine {
    fn provider(&self) -> PolishProvider {
        PolishProvider::None
    }

    async fn polish(&self, text: &str) -> Result<String, PolishError> {
        Ok(text.to_string())
    }
}
