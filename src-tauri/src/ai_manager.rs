//! AI Manager - Routes prompts to the selected local model
//!
//! Models downloaded by the app run on the built-in llama.cpp backend;
//! models installed in Ollama run through the Ollama API.

use crate::settings_manager::SettingsManager;
use crate::{local_inference, local_model, ollama};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use thiserror::Error;

const SYSTEM_PROMPT: &str = "You are a helpful note editor. Update the note content according to the user's request. \
Use Markdown formatting. Write in the same language as the note and the request. \
Output only the updated note content, without explanations or greetings.";

#[derive(Debug, Error)]
pub enum AiError {
    #[error("No AI model selected. Choose one in Settings.")]
    NoModel,
    #[error("{0}")]
    LocalModelError(#[from] local_model::LocalModelError),
    #[error("Local inference error: {0}")]
    LocalInferenceError(#[from] local_inference::LocalInferenceError),
    #[error("{0}")]
    OllamaError(#[from] ollama::OllamaError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamChunk {
    pub chunk: String,
    pub done: bool,
    pub gpu_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamError {
    pub code: String,
    pub message: String,
}

/// AI Manager handles routing prompts to the selected model
pub struct AiManager {
    client: Client,
    settings: Arc<SettingsManager>,
}

impl AiManager {
    pub fn new(settings: Arc<SettingsManager>) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self { client, settings }
    }

    /// Shared HTTP client (model downloads, Ollama API)
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Invoke AI with streaming response
    /// Emits 'ai-stream-chunk' events to the frontend
    pub async fn invoke_stream(
        &self,
        app: &AppHandle,
        prompt: &str,
        context: &str,
    ) -> Result<(), AiError> {
        let model_id = self.settings.get_active_model().ok_or(AiError::NoModel)?;

        let user_message = if context.trim().is_empty() {
            prompt.to_string()
        } else {
            format!("Current note:\n{}\n\nRequest: {}", context, prompt)
        };

        if let Some(name) = model_id.strip_prefix(local_model::OLLAMA_PREFIX) {
            ollama::chat_stream(app, &self.client, name, SYSTEM_PROMPT, &user_message).await?;
        } else {
            let model_path = local_model::app_model_path(&model_id)?;
            local_inference::run_local_inference(
                app,
                &model_path,
                SYSTEM_PROMPT,
                &user_message,
                self.settings.get_gpu_type(),
            )
            .await?;
        }

        Ok(())
    }
}
