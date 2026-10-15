//! Streaming STT abstraction.
//! - `mock`: instant cycling words (dev without mic/model/keys)
//! - `deepgram`: Nova-2 streaming REST (fastest cloud, ~200ms partials)
//! - `openai`: Whisper API chunked (slower, accurate)
//! - `local`: whisper.cpp sidecar stub — drop in `whisper-rs` + model file.
//!
//! Pipeline sends 1.5s windows (24k samples) with 0.25s overlap for continuity.
//! Swap providers in Settings without touching the pipeline.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Mock,
    Deepgram,
    Openai,
    Local,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Mock
    }
}

impl std::str::FromStr for Provider {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mock" => Ok(Provider::Mock),
            "deepgram" => Ok(Provider::Deepgram),
            "openai" => Ok(Provider::Openai),
            "local" => Ok(Provider::Local),
            _ => anyhow::bail!("unknown provider: {}", s),
        }
    }
}

pub struct Stt {
    pub provider: Provider,
    http: reqwest::Client,
    mock_i: usize,
}

impl Stt {
    pub fn new(provider: Provider) -> Self {
        Self { provider, http: reqwest::Client::new(), mock_i: 0 }
    }

    /// Transcribe one 16kHz mono window. Returns refined-ready raw text.
    /// Called every ~1.5s of voiced audio — must stay < window length for realtime.
    pub async fn transcribe(&mut self, pcm16k: &[f32]) -> Result<Transcription> {
        match self.provider {
            Provider::Mock => Ok(self.mock_transcribe()),
            Provider::Deepgram => self.deepgram(pcm16k).await,
            Provider::Openai => self.openai(pcm16k).await,
            Provider::Local => self.local(pcm16k).await,
        }
    }

    fn mock_transcribe(&mut self) -> Transcription {
        // Cycles demo words so UI + typer can be tested with no keys.
        const WORDS: &[&str] = &[
            "hello", "this", "is", "real", "time", "dictation", "at", "your", "cursor",
        ];
        let w = WORDS[self.mock_i % WORDS.len()];
        self.mock_i += 1;
        Transcription { text: w.to_string(), is_final: self.mock_i % 4 == 0 }
    }

    async fn deepgram(&self, pcm: &[f32]) -> Result<Transcription> {
        let key = std::env::var("DEEPGRAM_API_KEY").context("set DEEPGRAM_API_KEY")?;
        let bytes = pcm_to_wav_bytes(pcm);
        let resp = self
            .http
            .post("https://api.deepgram.com/v1/listen?model=nova-2&smart_format=true&interim_results=true")
            .header("Authorization", format!("Token {}", key))
            .header("Content-Type", "audio/wav")
            .body(bytes)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let text = resp
            .pointer("/results/channels/0/alternatives/0/transcript")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(Transcription { text, is_final: true })
    }

    async fn openai(&self, pcm: &[f32]) -> Result<Transcription> {
        let key = std::env::var("OPENAI_API_KEY").context("set OPENAI_API_KEY")?;
        let bytes = pcm_to_wav_bytes(pcm);
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name("chunk.wav")
            .mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part("file", part);
        let resp = self
            .http
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", key))
            .multipart(form)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let text = resp.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
        Ok(Transcription { text, is_final: true })
    }

    async fn local(&self, _pcm: &[f32]) -> Result<Transcription> {
        // Hook for whisper-rs offline:
        // 1. add `whisper-rs = "0.10"` + download ggml-tiny.en.bin to ~/.whispermic/models
        // 2. run full() here and return text. Falls back with clear error until then.
        let path = std::env::var("WHISPER_MODEL_PATH")
            .unwrap_or_else(|_| shellexpand("~/.whispermic/models/ggml-tiny.en.bin"));
        anyhow::bail!(
            "local STT not linked yet — model expected at {} — see apps/website/install.html #models",
            path
        )
    }
}

fn shellexpand(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    p.to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct Transcription {
    pub text: String,
    pub is_final: bool,
}

/// Minimal 16-bit WAV encoder (no extra dep, ~30 lines, fast).
fn pcm_to_wav_bytes(pcm: &[f32]) -> Vec<u8> {
    let n = pcm.len() as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + n * 2).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt len
    out.extend_from_slice(&1u16.to_le_bytes()); // pcm
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&(16000u32 * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(n * 2).to_le_bytes());
    for &s in pcm {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}
