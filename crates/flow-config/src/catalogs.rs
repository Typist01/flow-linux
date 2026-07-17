pub const LOCAL_WHISPER_MODELS: &[&str] = &["tiny.en", "base.en", "small.en", "medium.en"];

pub const OPENAI_STT_MODELS: &[&str] = &[
    "gpt-4o-mini-transcribe",
    "gpt-4o-transcribe",
    "whisper-1",
];

pub const OPENAI_STREAMING_STT_MODELS: &[&str] = &["gpt-realtime-whisper"];

pub const STREAMING_DELAYS: &[&str] = &["minimal", "low", "medium", "high", "xhigh"];

pub const OPENAI_POLISH_MODELS: &[&str] = &[
    "gpt-4o-mini",
    "gpt-4o",
    "gpt-4.1-mini",
    "gpt-4.1",
    "gpt-4.1-nano",
];

pub fn stt_provider_label(provider: crate::SttProvider) -> &'static str {
    match provider {
        crate::SttProvider::Local => "Local (Whisper.cpp)",
        crate::SttProvider::Openai => "OpenAI",
        crate::SttProvider::Deepgram => "Deepgram (coming soon)",
    }
}

pub fn stt_mode_label(mode: crate::SttMode) -> &'static str {
    match mode {
        crate::SttMode::Batch => "Batch (upload on release)",
        crate::SttMode::Streaming => "Streaming (live while held)",
    }
}

pub fn polish_provider_label(provider: crate::PolishProvider) -> &'static str {
    match provider {
        crate::PolishProvider::None => "Off",
        crate::PolishProvider::Ollama => "Ollama (coming soon)",
        crate::PolishProvider::Openai => "OpenAI",
    }
}
