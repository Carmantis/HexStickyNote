//! AI Manager - Routes prompts to different AI providers
//!
//! Supports streaming responses from OpenAI, Anthropic, Google Gemini, and local models.

use crate::ai_tools;
use crate::keyring_store::{AiProvider, KeyringStore};
use crate::settings_manager::SettingsManager;
use crate::{local_inference, local_model};
use directories::ProjectDirs;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("No API key configured for provider: {0}")]
    NoApiKey(String),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Provider not supported: {0}")]
    UnsupportedProvider(String),
    #[error("Local model error: {0}")]
    LocalModelError(#[from] local_model::LocalModelError),
    #[error("Local inference error: {0}")]
    LocalInferenceError(#[from] local_inference::LocalInferenceError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamChunk {
    pub chunk: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStreamError {
    pub code: String,
    pub message: String,
}

struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

// ============================================================================
// Persistent Storage Functions
// ============================================================================

/// Get the path to the active provider preference file
fn get_active_provider_file() -> Result<PathBuf, String> {
    let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
        .ok_or("Failed to determine project directories")?;

    let data_dir = proj_dirs.data_dir();
    fs::create_dir_all(data_dir).map_err(|e| format!("Failed to create data directory: {}", e))?;

    Ok(data_dir.join("active_provider.txt"))
}

/// Load the saved active provider from disk
fn load_active_provider() -> Option<AiProvider> {
    let file_path = get_active_provider_file().ok()?;

    if !file_path.exists() {
        return None;
    }

    let contents = fs::read_to_string(&file_path).ok()?;
    let provider_str = contents.trim();

    match AiProvider::from_str(provider_str) {
        Ok(provider) => {
            // Verify the API key still exists
            if KeyringStore::has_api_key(provider) {
                log::info!("Loaded active provider from disk: {}", provider_str);
                Some(provider)
            } else {
                log::warn!("Active provider {} has no API key configured", provider_str);
                None
            }
        }
        Err(e) => {
            log::warn!("Failed to parse saved provider '{}': {}", provider_str, e);
            None
        }
    }
}

/// Save the active provider to disk
fn save_active_provider(provider: AiProvider) -> Result<(), String> {
    let file_path = get_active_provider_file()?;

    fs::write(&file_path, provider.as_str())
        .map_err(|e| format!("Failed to save active provider: {}", e))?;

    log::debug!("Saved active provider to disk: {}", provider.as_str());
    Ok(())
}

// ============================================================================
// AI Manager
// ============================================================================

/// AI Manager handles routing prompts to different providers
pub struct AiManager {
    client: Client,
    active_provider: Arc<Mutex<Option<AiProvider>>>,
    settings: Arc<SettingsManager>,
}

impl AiManager {
    pub fn new(settings: Arc<SettingsManager>) -> Self {
        // Load the saved active provider from disk
        let saved_provider = load_active_provider();

        Self {
            client: Client::new(),
            active_provider: Arc::new(Mutex::new(saved_provider)),
            settings,
        }
    }

    pub async fn set_active_provider(&self, provider: AiProvider) {
        let mut active = self.active_provider.lock().await;
        *active = Some(provider);

        // Save to disk
        if let Err(e) = save_active_provider(provider) {
            log::error!("Failed to save active provider: {}", e);
        }

        log::info!("Active AI provider set to: {}", provider.as_str());
    }

    pub async fn get_active_provider(&self) -> Option<AiProvider> {
        *self.active_provider.lock().await
    }

    /// Invoke AI with streaming response
    /// Emits 'ai-stream-chunk' events to the frontend
    pub async fn invoke_stream(
        &self,
        app: &AppHandle,
        prompt: &str,
        context: &str,
    ) -> Result<(), AiError> {
        let provider = self
            .active_provider
            .lock()
            .await
            .ok_or_else(|| AiError::NoApiKey("No provider selected".to_string()))?;

        // Check if it's a local model
        if !provider.requires_api_key() {
            // Local model inference
            local_inference::run_local_inference(app, provider, prompt, context, Some(&self.settings)).await?;
            return Ok(());
        }

        // Cloud API inference
        let api_key = KeyringStore::get_api_key(provider)
            .map_err(|e| AiError::NoApiKey(e.to_string()))?;

        match provider {
            AiProvider::OpenAI => self.stream_openai(app, &api_key, prompt, context).await,
            AiProvider::Anthropic => self.stream_anthropic(app, &api_key, prompt, context).await,
            AiProvider::Google => self.stream_google(app, &api_key, prompt, context).await,
            _ => Err(AiError::UnsupportedProvider(format!("{:?}", provider))),
        }
    }

    async fn stream_openai(
        &self,
        app: &AppHandle,
        api_key: &str,
        prompt: &str,
        context: &str,
    ) -> Result<(), AiError> {
        let tools = ai_tools::get_all_tools();
        let model = self.settings.get_provider_model(AiProvider::OpenAI);

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful AI assistant for a sticky note application.
CRITICAL INSTRUCTION: When the user asks to create, update, or delete a note, you MUST use the provided tools (`create_note`, `update_note`, `delete_note`).
DO NOT rewrite the note content in your text response. Only use the tool.
If you use a tool, your text response should be empty or a very brief confirmation (e.g. 'Done').
Only output long text if you are answering a general question without modifying a note."
                },
                {
                    "role": "user",
                    "content": format!("Context (current card content):\n{}\n\nUser request: {}", context, prompt)
                }
            ],
            "tools": tools,
            "stream": true
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError(error_text));
        }

        let mut stream = response.bytes_stream();
        let mut pending_tool: Option<PendingToolCall> = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        // If there is a pending tool call that finished exactly at the end
                        if let Some(tool) = pending_tool.take() {
                            let _ = ai_tools::execute_tool(&tool.name, &tool.arguments);
                            // Signal frontend to refresh data
                            app.emit("refresh-required", ()).ok();
                        }

                        app.emit("ai-stream-chunk", AiStreamChunk {
                            chunk: String::new(),
                            done: true,
                        }).ok();
                        return Ok(());
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        let delta = &json["choices"][0]["delta"];

                        // 1. Handle normal text content
                        if let Some(content) = delta["content"].as_str() {
                            app.emit("ai-stream-chunk", AiStreamChunk {
                                chunk: content.to_string(),
                                done: false,
                            }).ok();
                        }

                        // 2. Handle Tool Calls
                        if let Some(tool_calls) = delta["tool_calls"].as_array() {
                            for call in tool_calls {
                                let _index = call["index"].as_u64().unwrap_or(0);
                                
                                // New tool call starting (assuming index 0 for simplicity in streaming one tool)
                                if let Some(id) = call["id"].as_str() {
                                    // If we had a previous one, execute it now (though OpenAI usually finishes one before starting next?)
                                    // In streaming, 'id' is sent only in the first chunk of the tool call.
                                    pending_tool = Some(PendingToolCall {
                                        id: id.to_string(),
                                        name: String::new(),
                                        arguments: String::new(),
                                    });
                                }

                                if let Some(function) = call["function"].as_object() {
                                    if let Some(pt) = &mut pending_tool {
                                        if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                            pt.name.push_str(name);
                                        }
                                        if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                                            pt.arguments.push_str(args);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Check finish_reason to execute tool
                        if let Some(finish_reason) = json["choices"][0]["finish_reason"].as_str() {
                            if finish_reason == "tool_calls" {
                                if let Some(tool) = pending_tool.take() {
                                    let _ = ai_tools::execute_tool(&tool.name, &tool.arguments);
                                    // Signal frontend to refresh data
                                    app.emit("refresh-required", ()).ok();
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn stream_anthropic(
        &self,
        app: &AppHandle,
        api_key: &str,
        prompt: &str,
        context: &str,
    ) -> Result<(), AiError> {
        let model = self.settings.get_provider_model(AiProvider::Anthropic);

        let body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": [
                {
                    "role": "user",
                    "content": format!("Context (current card content):\n{}\n\nUser request: {}", context, prompt)
                }
            ],
            "stream": true
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError(error_text));
        }

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        let event_type = json["type"].as_str().unwrap_or("");

                        match event_type {
                            "content_block_delta" => {
                                if let Some(text) = json["delta"]["text"].as_str() {
                                    app.emit("ai-stream-chunk", AiStreamChunk {
                                        chunk: text.to_string(),
                                        done: false,
                                    }).ok();
                                }
                            }
                            "message_stop" => {
                                app.emit("ai-stream-chunk", AiStreamChunk {
                                    chunk: String::new(),
                                    done: true,
                                }).ok();
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn stream_google(
        &self,
        app: &AppHandle,
        api_key: &str,
        prompt: &str,
        context: &str,
    ) -> Result<(), AiError> {
        let model = self.settings.get_provider_model(AiProvider::Google);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            model, api_key
        );

        let body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": format!("SYSTEM: You are a text editor. Your goal is to update the note content based on the user request. Output ONLY the full updated note content. Do not output conversational text.\n\nContext (current content):\n{}\n\nUser request: {}", context, prompt)
                        }
                    ]
                }
            ]
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError(error_text));
        }

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                            app.emit("ai-stream-chunk", AiStreamChunk {
                                chunk: text.to_string(),
                                done: false,
                            }).ok();
                        }

                        if json["candidates"][0]["finishReason"].as_str().is_some() {
                            app.emit("ai-stream-chunk", AiStreamChunk {
                                chunk: String::new(),
                                done: true,
                            }).ok();
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
