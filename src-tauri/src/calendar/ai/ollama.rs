use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::calendar::models::error::CalError;
use crate::local_model::OLLAMA_PREFIX;
use crate::settings_manager::SettingsManager;

/// Ollama client for calendar AI. Uses the Ollama model selected in
/// HexStickyNote's settings, so notes and calendar share one model choice.
#[derive(Clone)]
pub struct OllamaClient {
    settings: Arc<SettingsManager>,
    http: Client,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    /// Thinking models would otherwise put their reasoning into the answer
    think: bool,
}

#[derive(Deserialize)]
struct GenerateChunk {
    response: String,
    done: bool,
}

impl OllamaClient {
    pub fn new(settings: Arc<SettingsManager>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self { settings, http }
    }

    /// The selected Ollama model; calendar AI does not run app-downloaded models
    pub fn model(&self) -> Result<String, CalError> {
        self.settings
            .get_active_model()
            .and_then(|id| id.strip_prefix(OLLAMA_PREFIX).map(str::to_string))
            .ok_or_else(|| {
                CalError::Ai("Calendar AI needs an Ollama model. Select one in Settings.".to_string())
            })
    }

    fn request(&self, prompt: &str, stream: bool) -> Result<reqwest::RequestBuilder, CalError> {
        let body = GenerateRequest {
            model: self.model()?,
            prompt: prompt.to_string(),
            stream,
            think: false,
        };
        Ok(self
            .http
            .post(format!("{}/api/generate", crate::ollama::base_url()))
            .json(&body))
    }

    /// Generate a response from Ollama and return the full text.
    pub async fn generate(&self, prompt: &str) -> Result<String, CalError> {
        let resp = self.request(prompt, false)?.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CalError::Ai(format!("Ollama returned {}: {}", status, text)));
        }

        let chunk: GenerateChunk = resp.json().await?;
        Ok(chunk.response)
    }

    /// Stream a response token by token, emitting each token as a Tauri event.
    /// The frontend listens to the "ai_token" event and the "ai_done" event.
    pub async fn stream_to_window(&self, prompt: &str, app: &AppHandle) -> Result<(), CalError> {
        let resp = self.request(prompt, true)?.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CalError::Ai(format!("Ollama returned {}: {}", status, text)));
        }

        // Newline-delimited JSON; a line may span several network chunks
        let mut stream = resp.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();

        while let Some(item) = stream.next().await {
            buffer.extend_from_slice(&item.map_err(|e| CalError::Ai(e.to_string()))?);

            while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=pos).collect();
                let Ok(chunk) = serde_json::from_slice::<GenerateChunk>(&line) else {
                    continue;
                };
                if !chunk.response.is_empty() {
                    app.emit("ai_token", &chunk.response)
                        .map_err(|e| CalError::Ai(e.to_string()))?;
                }
                if chunk.done {
                    app.emit("ai_done", ()).map_err(|e| CalError::Ai(e.to_string()))?;
                    return Ok(());
                }
            }
        }

        app.emit("ai_done", ()).map_err(|e| CalError::Ai(e.to_string()))?;

        Ok(())
    }
}
