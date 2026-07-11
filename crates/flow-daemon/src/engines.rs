use flow_config::Config;
use flow_polish::PolishEngine;
use flow_stt::SttEngine;
use std::sync::Arc;
use thiserror::Error;

pub struct EngineSet {
    pub config: Arc<Config>,
    pub stt: Arc<dyn SttEngine>,
    pub polish: Arc<dyn PolishEngine>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("STT engine error: {0}")]
    Stt(#[from] flow_stt::SttError),
    #[error("polish engine error: {0}")]
    Polish(#[from] flow_polish::PolishError),
}

pub fn build_engines(config: &Config) -> Result<EngineSet, EngineError> {
    let stt = flow_stt::build_stt_engine(config)?;
    let polish = flow_polish::build_polish_engine(config)?;
    Ok(EngineSet {
        config: Arc::new(config.clone()),
        stt,
        polish,
    })
}
