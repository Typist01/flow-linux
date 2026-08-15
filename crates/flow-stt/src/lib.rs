mod download;
mod openai;
mod realtime;
mod wav;
mod whisper;

use flow_config::{Config, SttProvider};
use std::sync::Arc;
use thiserror::Error;

pub use openai::OpenAiSttEngine;
pub use realtime::{RealtimeError, RealtimeTranscriptionSession, StreamingEvent};
pub use whisper::WhisperEngine;

/// Speech-to-text backend. Local engines use `spawn_blocking`; cloud engines use HTTP.
#[async_trait::async_trait]
pub trait SttEngine: Send + Sync {
    fn provider(&self) -> SttProvider;
    async fn transcribe(&self, samples: &[f32]) -> Result<String, SttError>;
}

#[derive(Debug, Error)]
pub enum SttError {
    #[error("model not found at {0}")]
    ModelNotFound(String),
    #[error("whisper: {0}")]
    Whisper(#[from] whisper_rs::WhisperError),
    #[error("provider {0:?} is not implemented yet")]
    NotImplemented(SttProvider),
    #[error("cloud STT: {0}")]
    Cloud(String),
    #[error("realtime STT: {0}")]
    Realtime(#[from] RealtimeError),
}

pub fn build_stt_engine(config: &Config) -> Result<Arc<dyn SttEngine>, SttError> {
    // Streaming uses RealtimeTranscriptionSession directly; batch engines only here.
    match config.stt.provider {
        SttProvider::Local => Ok(Arc::new(WhisperEngine::new(config)?)),
        SttProvider::Openai => Ok(Arc::new(OpenAiSttEngine::new(config)?)),
        SttProvider::Deepgram => Err(SttError::NotImplemented(SttProvider::Deepgram)),
    }
}

pub fn ensure_local_model_exists(config: &Config) -> bool {
    WhisperEngine::ensure_model_exists(&config.model_path())
}

pub use download::{download_local_model, model_download_url};
pub use wav::samples_to_wav;
