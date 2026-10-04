//! Ollama integration
//!
//! Lists the models installed in a locally running Ollama and streams chat
//! responses through its HTTP API. The API is used instead of reading Ollama's
//! model files directly because the system service keeps them in a directory
//! other users cannot read, and Ollama supports newer model architectures.

use crate::ai_manager::AiStreamChunk;
use crate::local_model::{LocalModelInfo, ModelSource, OLLAMA_PREFIX};
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

const DEFAULT_HOST: &str = "127.0.0.1:11434";

#[derive(Debug, Error)]
pub enum OllamaError {
    #[error("Ollama is not running ({0})")]
    Unavailable(String),
    #[error("Ollama error: {0}")]
    Api(String),
}

impl From<reqwest::Error> for OllamaError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() {
            OllamaError::Unavailable(base_url())
        } else {
            OllamaError::Api(e.to_string())
        }
    }
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    details: Option<OllamaModelDetails>,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelDetails {
    #[serde(default)]
    parameter_size: Option<String>,
    #[serde(default)]
    quantization_level: Option<String>,
}

/// Base URL of the Ollama API, honouring OLLAMA_HOST like the Ollama CLI does
pub fn base_url() -> String {
    let host = std::env::var("OLLAMA_HOST")
        .ok()
        .map(|h| h.trim().trim_end_matches('/').to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| DEFAULT_HOST.to_string());

    let (scheme, rest) = match host.split_once("://") {
        Some((scheme, rest)) => (scheme.to_string(), rest.to_string()),
        None => ("http".to_string(), host),
    };
    // A server bound to all interfaces is reached through loopback
    let rest = rest.replacen("0.0.0.0", "127.0.0.1", 1);
    let rest = if rest.rsplit_once(':').map_or(true, |(_, p)| p.parse::<u16>().is_err()) {
        format!("{}:11434", rest)
    } else {
        rest
    };

    format!("{}://{}", scheme, rest)
}

async fn fetch_models(client: &Client) -> Result<Vec<OllamaModel>, OllamaError> {
    let response = client
        .get(format!("{}/api/tags", base_url()))
        .timeout(Duration::from_secs(2))
        .send()
        .await?
        .error_for_status()?;

    let tags: TagsResponse = response.json().await?;
    Ok(tags.models)
}

/// List models installed in the local Ollama
pub async fn list_models(client: &Client) -> Result<Vec<LocalModelInfo>, OllamaError> {
    let mut models: Vec<LocalModelInfo> = fetch_models(client)
        .await?
        .into_iter()
        .map(|m| {
            let details = m.details.and_then(|d| {
                let parts: Vec<String> = [d.parameter_size, d.quantization_level]
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .collect();
                (!parts.is_empty()).then(|| parts.join(" · "))
            });
            LocalModelInfo {
                id: format!("{}{}", OLLAMA_PREFIX, m.name),
                name: m.name,
                source: ModelSource::Ollama,
                size: m.size,
                details,
            }
        })
        .collect();

    models.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(models)
}

/// Capabilities Ollama reports for an installed model, e.g. "tools", "thinking"
pub async fn capabilities(client: &Client, model: &str) -> Result<Vec<String>, OllamaError> {
    Ok(fetch_models(client)
        .await?
        .into_iter()
        .find(|m| m.name == model)
        .map(|m| m.capabilities)
        .unwrap_or_default())
}

/// Turn an unsuccessful response into Ollama's own error message
async fn api_error(response: reqwest::Response) -> OllamaError {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v["error"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{} {}", status, text));
    OllamaError::Api(message)
}

/// One non-streaming /api/chat call; returns the assistant message
pub async fn chat(client: &Client, body: serde_json::Value) -> Result<serde_json::Value, OllamaError> {
    let response = client
        .post(format!("{}/api/chat", base_url()))
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(api_error(response).await);
    }

    let mut reply: serde_json::Value = response.json().await?;
    Ok(reply["message"].take())
}

/// Stream a chat response from Ollama, emitting 'ai-stream-chunk' events
pub async fn chat_stream(
    app: &AppHandle,
    client: &Client,
    model: &str,
    system_prompt: &str,
    user_message: &str,
) -> Result<(), OllamaError> {
    // Thinking models would otherwise write their reasoning into the note
    let supports_thinking = capabilities(client, model).await?.iter().any(|c| c == "thinking");

    let mut body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ]
    });
    if supports_thinking {
        body["think"] = serde_json::json!(false);
    }

    log::info!("Starting Ollama chat with model {}", model);
    let response = client
        .post(format!("{}/api/chat", base_url()))
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(api_error(response).await);
    }

    let emit = |chunk: String, done: bool| {
        app.emit(
            "ai-stream-chunk",
            AiStreamChunk {
                chunk,
                done,
                gpu_info: Some("Ollama".to_string()),
            },
        )
        .ok();
    };

    // The response is newline-delimited JSON; a line may span several network chunks
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    while let Some(chunk) = stream.next().await {
        buffer.extend_from_slice(&chunk?);

        while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let json: serde_json::Value = serde_json::from_str(line)
                .map_err(|e| OllamaError::Api(format!("invalid response: {}", e)))?;

            if let Some(error) = json["error"].as_str() {
                return Err(OllamaError::Api(error.to_string()));
            }
            if let Some(content) = json["message"]["content"].as_str() {
                if !content.is_empty() {
                    emit(content.to_string(), false);
                }
            }
            if json["done"].as_bool() == Some(true) {
                emit(String::new(), true);
                return Ok(());
            }
        }
    }

    emit(String::new(), true);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_base_url_from_ollama_host() {
        let cases = [
            (None, "http://127.0.0.1:11434"),
            (Some("0.0.0.0"), "http://127.0.0.1:11434"),
            (Some("0.0.0.0:8080"), "http://127.0.0.1:8080"),
            (Some("https://ollama.example.com:443/"), "https://ollama.example.com:443"),
            (Some("myhost"), "http://myhost:11434"),
        ];
        for (env, expected) in cases {
            match env {
                Some(v) => std::env::set_var("OLLAMA_HOST", v),
                None => std::env::remove_var("OLLAMA_HOST"),
            }
            assert_eq!(base_url(), expected, "OLLAMA_HOST={:?}", env);
        }
        std::env::remove_var("OLLAMA_HOST");
    }
}
