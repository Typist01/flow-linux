use crate::{SttEngine, SttError};
use flow_config::{Config, SttProvider};
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperEngine {
    ctx: Arc<WhisperContext>,
    language: String,
}

impl WhisperEngine {
    pub fn new(config: &Config) -> Result<Self, SttError> {
        let path = config.model_path();
        if !path.exists() {
            return Err(SttError::ModelNotFound(path.display().to_string()));
        }

        tracing::info!(model = %path.display(), "loading whisper model");
        let ctx = WhisperContext::new_with_params(
            path.to_str().ok_or_else(|| {
                SttError::ModelNotFound("invalid model path encoding".to_string())
            })?,
            WhisperContextParameters::default(),
        )?;

        Ok(Self {
            ctx: Arc::new(ctx),
            language: config.language().to_string(),
        })
    }

    pub fn ensure_model_exists(path: &std::path::Path) -> bool {
        path.exists()
    }

    fn transcribe_blocking(&self, samples: &[f32]) -> Result<String, SttError> {
        let start = std::time::Instant::now();
        let mut state = self.ctx.create_state()?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(&self.language));
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_timestamps(false);

        state.full(params, samples)?;

        let n = state.full_n_segments();
        let mut text = String::new();
        for i in 0..n {
            if let Some(segment) = state.get_segment(i) {
                text.push_str(segment.to_str()?);
            }
        }

        let transcript = text.trim().to_string();
        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            chars = transcript.len(),
            provider = "local",
            "transcription complete"
        );
        Ok(transcript)
    }
}

#[async_trait::async_trait]
impl SttEngine for WhisperEngine {
    fn provider(&self) -> SttProvider {
        SttProvider::Local
    }

    async fn transcribe(&self, samples: &[f32]) -> Result<String, SttError> {
        let engine = Self {
            ctx: Arc::clone(&self.ctx),
            language: self.language.clone(),
        };
        let samples = samples.to_vec();
        tokio::task::spawn_blocking(move || engine.transcribe_blocking(&samples))
            .await
            .map_err(|e| SttError::Cloud(e.to_string()))?
    }
}
