use crate::engines::EngineSet;
use flow_inject::inject_text;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("transcription failed: {0}")]
    Stt(#[from] flow_stt::SttError),
    #[error("polish failed: {0}")]
    Polish(#[from] flow_polish::PolishError),
    #[error("injection failed: {0}")]
    Inject(String),
    #[error("realtime STT: {0}")]
    Realtime(#[from] flow_stt::RealtimeError),
}

pub async fn run_dictation_pipeline(
    engines: &EngineSet,
    samples: Vec<f32>,
) -> Result<String, PipelineError> {
    let raw = engines.stt.transcribe(&samples).await?;
    finish_transcript(engines, raw).await
}

pub async fn finish_transcript(
    engines: &EngineSet,
    raw: String,
) -> Result<String, PipelineError> {
    if raw.is_empty() {
        return Ok(String::new());
    }

    tracing::info!(%raw, "transcript");

    let final_text = match engines.polish.polish(&raw).await {
        Ok(text) => text,
        Err(flow_polish::PolishError::NotImplemented(provider)) => {
            tracing::warn!(?provider, "polish provider not implemented — using raw transcript");
            raw.clone()
        }
        Err(e) => {
            tracing::warn!(error = %e, "polish failed — using raw transcript");
            raw.clone()
        }
    };

    inject_text(&final_text, engines.config.inject_via_paste())
        .await
        .map_err(|e| PipelineError::Inject(e.to_string()))?;

    Ok(final_text)
}
