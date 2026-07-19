use crate::{samples_to_wav, SttEngine, SttError};
use flow_config::{Config, SttProvider, SAMPLE_RATE};
use flow_secrets::resolve_openai_api_key;
use reqwest::multipart;
use serde::Deserialize;

pub struct OpenAiSttEngine {
    client: reqwest::Client,
    model: String,
    language: String,
    api_key_env: String,
    transcriptions_url: String,
}

impl OpenAiSttEngine {
    pub fn new(config: &Config) -> Result<Self, SttError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| SttError::Cloud(e.to_string()))?,
            model: config.stt.openai_model.clone(),
            language: config.stt.language.clone(),
            api_key_env: config.stt.openai_api_key_env.clone(),
            transcriptions_url: openai_endpoint(
                &config.stt.openai_api_base,
                "audio/transcriptions",
            ),
        })
    }
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

#[async_trait::async_trait]
impl SttEngine for OpenAiSttEngine {
    fn provider(&self) -> SttProvider {
        SttProvider::Openai
    }

    async fn transcribe(&self, samples: &[f32]) -> Result<String, SttError> {
        let start = std::time::Instant::now();
        let api_key = resolve_openai_api_key(&self.api_key_env)
            .map_err(|e| SttError::Cloud(e.to_string()))?;
        let wav = samples_to_wav(samples, SAMPLE_RATE);

        let file_part = multipart::Part::bytes(wav)
            .file_name("flow-dictation.wav")
            .mime_str("audio/wav")
            .map_err(|e| SttError::Cloud(e.to_string()))?;

        let form = multipart::Form::new()
            .text("model", self.model.clone())
            .text("language", self.language.clone())
            .text("response_format", "json")
            .part("file", file_part);

        let response = self
            .client
            .post(&self.transcriptions_url)
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| SttError::Cloud(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SttError::Cloud(format!("OpenAI STT {status}: {body}")));
        }

        let payload: TranscriptionResponse = response
            .json()
            .await
            .map_err(|e| SttError::Cloud(e.to_string()))?;

        let transcript = payload.text.trim().to_string();
        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            chars = transcript.len(),
            model = %self.model,
            provider = "openai",
            "transcription complete"
        );
        Ok(transcript)
    }
}

fn openai_endpoint(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}
