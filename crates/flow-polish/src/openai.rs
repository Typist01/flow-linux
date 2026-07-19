use crate::{PolishEngine, PolishError};
use flow_config::{Config, PolishProvider};
use flow_secrets::resolve_openai_api_key;
use serde::{Deserialize, Serialize};

const SYSTEM_PROMPT: &str = "\
You are a dictation editor. Fix capitalization and punctuation. \
Remove filler words (um, uh, like, you know). Do not change meaning. \
Do not add content. Output only the corrected text.";

pub struct OpenAiPolishEngine {
    client: reqwest::Client,
    model: String,
    api_key_env: String,
    chat_url: String,
}

impl OpenAiPolishEngine {
    pub fn new(config: &Config) -> Result<Self, PolishError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| PolishError::Request(e.to_string()))?,
            model: config.polish.openai_model.clone(),
            api_key_env: config.polish.openai_api_key_env.clone(),
            chat_url: openai_endpoint(&config.polish.openai_api_base, "chat/completions"),
        })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

#[async_trait::async_trait]
impl PolishEngine for OpenAiPolishEngine {
    fn provider(&self) -> PolishProvider {
        PolishProvider::Openai
    }

    async fn polish(&self, text: &str) -> Result<String, PolishError> {
        if text.chars().count() < 3 {
            return Ok(text.to_string());
        }

        let api_key = resolve_openai_api_key(&self.api_key_env)
            .map_err(|e| PolishError::Request(e.to_string()))?;

        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: SYSTEM_PROMPT,
                },
                ChatMessage {
                    role: "user",
                    content: text,
                },
            ],
            temperature: 0.2,
        };

        let response = self
            .client
            .post(&self.chat_url)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| PolishError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(PolishError::Request(format!(
                "OpenAI polish {status}: {body}"
            )));
        }

        let payload: ChatResponse = response
            .json()
            .await
            .map_err(|e| PolishError::Request(e.to_string()))?;

        let polished = payload
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                PolishError::Request("OpenAI returned empty polish response".to_string())
            })?;

        tracing::info!(chars = polished.len(), model = %self.model, "polish complete");
        Ok(polished)
    }
}

fn openai_endpoint(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}
