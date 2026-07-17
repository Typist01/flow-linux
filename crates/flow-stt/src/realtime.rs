use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flow_audio::{resample_linear, samples_to_pcm16_bytes};
use flow_config::{Config, STREAMING_SAMPLE_RATE};
use flow_secrets::resolve_openai_api_key;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
};

const REALTIME_URL: &str = "wss://api.openai.com/v1/realtime?intent=transcription";

#[derive(Debug, Clone)]
pub enum StreamingEvent {
    Delta(String),
    Completed(String),
    SessionReady,
    Error(String),
}

#[derive(Debug, Error)]
pub enum RealtimeError {
    #[error("cloud STT: {0}")]
    Cloud(String),
    #[error("websocket: {0}")]
    Websocket(String),
    #[error("session closed before completion")]
    ClosedEarly,
    #[error("timed out waiting for final transcript")]
    Timeout,
}

pub struct RealtimeTranscriptionSession {
    write_tx: mpsc::UnboundedSender<Value>,
    event_rx: mpsc::UnboundedReceiver<StreamingEvent>,
    join: Option<tokio::task::JoinHandle<()>>,
    source_sample_rate: u32,
}

impl RealtimeTranscriptionSession {
    pub async fn connect(config: &Config, source_sample_rate: u32) -> Result<Self, RealtimeError> {
        let api_key = resolve_openai_api_key(&config.stt.openai_api_key_env)
            .map_err(|e| RealtimeError::Cloud(e.to_string()))?;

        let mut request = REALTIME_URL
            .into_client_request()
            .map_err(|e| RealtimeError::Websocket(e.to_string()))?;
        let headers = request.headers_mut();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| RealtimeError::Websocket(e.to_string()))?,
        );

        let (ws, _) = connect_async(request)
            .await
            .map_err(|e| RealtimeError::Websocket(e.to_string()))?;
        let (mut write, mut read) = ws.split();

        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Value>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<StreamingEvent>();

        let model = config.stt.streaming_model.clone();
        let language = config.stt.language.clone();
        let delay = config.stt.streaming_delay.clone();

        let session_update = json!({
            "type": "session.update",
            "session": {
                "type": "transcription",
                "audio": {
                    "input": {
                        "format": {
                            "type": "audio/pcm",
                            "rate": STREAMING_SAMPLE_RATE
                        },
                        "transcription": {
                            "model": model,
                            "language": language,
                            "delay": delay
                        },
                        "turn_detection": null
                    }
                }
            }
        });

        write
            .send(Message::Text(session_update.to_string().into()))
            .await
            .map_err(|e| RealtimeError::Websocket(e.to_string()))?;

        let join = tokio::spawn(async move {
            loop {
                tokio::select! {
                    outgoing = write_rx.recv() => {
                        match outgoing {
                            Some(payload) => {
                                if write
                                    .send(Message::Text(payload.to_string().into()))
                                    .await
                                    .is_err()
                                {
                                    let _ = event_tx.send(StreamingEvent::Error(
                                        "failed to send websocket message".into(),
                                    ));
                                    break;
                                }
                            }
                            None => {
                                let _ = write.send(Message::Close(None)).await;
                                break;
                            }
                        }
                    }
                    incoming = read.next() => {
                        match incoming {
                            Some(Ok(Message::Text(text))) => {
                                handle_server_event(&text, &event_tx);
                            }
                            Some(Ok(Message::Ping(payload))) => {
                                let _ = write.send(Message::Pong(payload)).await;
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                let _ = event_tx.send(StreamingEvent::Error(
                                    "websocket closed".into(),
                                ));
                                break;
                            }
                            Some(Ok(_)) => {}
                            Some(Err(e)) => {
                                let _ = event_tx.send(StreamingEvent::Error(e.to_string()));
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            write_tx,
            event_rx,
            join: Some(join),
            source_sample_rate,
        })
    }

    pub fn append_pcm_f32(&self, samples: &[f32]) -> Result<(), RealtimeError> {
        if samples.is_empty() {
            return Ok(());
        }

        let resampled = resample_linear(samples, self.source_sample_rate, STREAMING_SAMPLE_RATE);
        let pcm = samples_to_pcm16_bytes(&resampled);
        let audio = BASE64.encode(pcm);
        self.write_tx
            .send(json!({
                "type": "input_audio_buffer.append",
                "audio": audio
            }))
            .map_err(|_| RealtimeError::ClosedEarly)?;
        Ok(())
    }

    pub fn commit(&self) -> Result<(), RealtimeError> {
        self.write_tx
            .send(json!({ "type": "input_audio_buffer.commit" }))
            .map_err(|_| RealtimeError::ClosedEarly)?;
        Ok(())
    }

    pub async fn recv_event(&mut self) -> Option<StreamingEvent> {
        self.event_rx.recv().await
    }

    pub async fn wait_for_completed(&mut self, timeout: Duration) -> Result<String, RealtimeError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut last_delta = String::new();

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                if !last_delta.is_empty() {
                    return Ok(last_delta);
                }
                return Err(RealtimeError::Timeout);
            }

            match tokio::time::timeout(remaining, self.event_rx.recv()).await {
                Ok(Some(StreamingEvent::Completed(text))) => {
                    return Ok(if text.is_empty() { last_delta } else { text });
                }
                Ok(Some(StreamingEvent::Delta(delta))) => {
                    last_delta.push_str(&delta);
                    tracing::info!(%delta, "streaming delta");
                }
                Ok(Some(StreamingEvent::SessionReady)) => {}
                Ok(Some(StreamingEvent::Error(e))) => {
                    if !last_delta.is_empty() {
                        tracing::warn!(error = %e, "streaming error after partial transcript");
                        return Ok(last_delta);
                    }
                    return Err(RealtimeError::Cloud(e));
                }
                Ok(None) => {
                    if !last_delta.is_empty() {
                        return Ok(last_delta);
                    }
                    return Err(RealtimeError::ClosedEarly);
                }
                Err(_) => {
                    if !last_delta.is_empty() {
                        return Ok(last_delta);
                    }
                    return Err(RealtimeError::Timeout);
                }
            }
        }
    }

    pub async fn close(mut self) {
        drop(self.write_tx);
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }
}

fn handle_server_event(text: &str, event_tx: &mpsc::UnboundedSender<StreamingEvent>) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let Some(event_type) = value.get("type").and_then(Value::as_str) else {
        return;
    };

    match event_type {
        "session.created" | "session.updated" => {
            let _ = event_tx.send(StreamingEvent::SessionReady);
        }
        "conversation.item.input_audio_transcription.delta"
        | "transcript.text.delta" => {
            if let Some(delta) = extract_delta(&value) {
                let _ = event_tx.send(StreamingEvent::Delta(delta));
            }
        }
        "conversation.item.input_audio_transcription.completed"
        | "transcript.text.done" => {
            if let Some(transcript) = extract_completed(&value) {
                let _ = event_tx.send(StreamingEvent::Completed(transcript));
            }
        }
        "error" => {
            let message = value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| value.get("message").and_then(Value::as_str))
                .unwrap_or("unknown realtime error")
                .to_string();
            let _ = event_tx.send(StreamingEvent::Error(message));
        }
        _ => {
            tracing::debug!(event_type, "realtime event");
        }
    }
}

fn extract_delta(value: &Value) -> Option<String> {
    value
        .get("delta")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .pointer("/transcript")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn extract_completed(value: &Value) -> Option<String> {
    value
        .get("transcript")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .or_else(|| {
            #[derive(Deserialize)]
            struct Wrapper {
                text: Option<String>,
                transcript: Option<String>,
            }
            serde_json::from_value::<Wrapper>(value.clone())
                .ok()
                .and_then(|w| w.transcript.or(w.text))
                .map(|s| s.trim().to_string())
        })
}
