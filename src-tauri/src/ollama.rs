//! Ollama integration
//!
//! All AI in HexStickyNote runs on a locally running Ollama: this module lists
//! its models, pulls new ones from the Ollama library and makes chat calls.
//! The HTTP API is used instead of reading Ollama's model files directly
//! because the system service keeps them in a directory other users cannot read.

use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

const DEFAULT_HOST: &str = "127.0.0.1:11434";

/// Prefix of model ids stored in settings ("ollama:<name>")
pub const OLLAMA_PREFIX: &str = "ollama:";

static PULL_ACTIVE: AtomicBool = AtomicBool::new(false);
static PULL_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Error)]
pub enum OllamaError {
    #[error("Ollama is not running ({0})")]
    Unavailable(String),
    #[error("Ollama error: {0}")]
    Api(String),
    #[error("Invalid model name: {0}")]
    InvalidName(String),
    #[error("Another model download is already in progress")]
    PullInProgress,
    #[error("Download cancelled")]
    Cancelled,
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

/// A model installed in Ollama, as shown in Settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// "ollama:<name>", the id stored as the active model
    pub id: String,
    pub name: String,
    pub size: Option<u64>,
    /// Short human-readable details, e.g. "27.3B · Q4_K_M"
    pub details: Option<String>,
    /// Whether the assistant can use it (tool calling)
    pub supports_tools: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullProgress {
    pub name: String,
    pub status: String,
    pub completed: u64,
    pub total: u64,
    pub percentage: f64,
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
pub async fn list_models(client: &Client) -> Result<Vec<ModelInfo>, OllamaError> {
    let mut models: Vec<ModelInfo> = fetch_models(client)
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
            ModelInfo {
                id: format!("{}{}", OLLAMA_PREFIX, m.name),
                supports_tools: m.capabilities.iter().any(|c| c == "tools"),
                name: m.name,
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

// ============================================================================
// Pulling models
// ============================================================================

/// Turn what a user typed or pasted into a name Ollama can pull:
/// `llama3.2:3b`, `user/model`, `hf.co/user/repo:Q4_K_M`, or an ollama.com URL.
pub fn normalize_model_name(input: &str) -> Result<String, OllamaError> {
    let invalid = || OllamaError::InvalidName(input.trim().to_string());

    let mut name = input.trim();
    for prefix in ["https://", "http://"] {
        name = name.strip_prefix(prefix).unwrap_or(name);
    }
    for prefix in ["www.ollama.com/", "ollama.com/", "registry.ollama.ai/"] {
        name = name.strip_prefix(prefix).unwrap_or(name);
    }
    let name = name.strip_prefix("library/").unwrap_or(name).trim_end_matches('/');
    let name = match name.strip_prefix("huggingface.co/") {
        Some(rest) => format!("hf.co/{}", rest),
        None => name.to_string(),
    };

    let valid_chars = name.chars().all(|c| c.is_ascii_alphanumeric() || "._-/:".contains(c));
    let valid_parts = name.split(['/', ':']).all(|part| !part.is_empty() && part != "." && part != "..");
    if name.is_empty() || !valid_chars || !valid_parts || name.matches(':').count() > 1 {
        return Err(invalid());
    }
    Ok(name)
}

/// Resets the "pull in progress" flag when the pull ends in any way
struct PullGuard;

impl Drop for PullGuard {
    fn drop(&mut self) {
        PULL_ACTIVE.store(false, Ordering::SeqCst);
    }
}

/// Ask the running pull to stop
pub fn cancel_pull() {
    PULL_CANCELLED.store(true, Ordering::SeqCst);
}

/// Download a model into Ollama (`ollama pull`).
/// Emits 'model-pull-progress' events; returns the model id ("ollama:<name>").
pub async fn pull_model(app: &AppHandle, client: &Client, input: &str) -> Result<String, OllamaError> {
    let name = normalize_model_name(input)?;

    if PULL_ACTIVE.swap(true, Ordering::SeqCst) {
        return Err(OllamaError::PullInProgress);
    }
    let _guard = PullGuard;
    PULL_CANCELLED.store(false, Ordering::SeqCst);

    log::info!("Pulling model {} into Ollama", name);
    let response = client
        .post(format!("{}/api/pull", base_url()))
        .json(&serde_json::json!({ "model": name, "stream": true }))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(api_error(response).await);
    }

    // Progress per layer; the total is the sum over the layers seen so far
    let mut layers: HashMap<String, (u64, u64)> = HashMap::new();
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    let mut last_percentage = -1.0;

    while let Some(chunk) = stream.next().await {
        if PULL_CANCELLED.load(Ordering::SeqCst) {
            // Dropping the response closes the connection, which stops the pull
            return Err(OllamaError::Cancelled);
        }
        buffer.extend_from_slice(&chunk?);

        while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buffer.drain(..=pos).collect();
            let Ok(update) = serde_json::from_slice::<serde_json::Value>(&line) else {
                continue;
            };

            if let Some(error) = update["error"].as_str() {
                return Err(OllamaError::Api(error.to_string()));
            }
            let status = update["status"].as_str().unwrap_or_default().to_string();
            if status == "success" {
                log::info!("Model {} pulled", name);
                return Ok(format!("{}{}", OLLAMA_PREFIX, name));
            }

            if let (Some(digest), Some(total)) = (update["digest"].as_str(), update["total"].as_u64()) {
                let completed = update["completed"].as_u64().unwrap_or(0);
                layers.insert(digest.to_string(), (completed, total));
            }
            let completed: u64 = layers.values().map(|(c, _)| c).sum();
            let total: u64 = layers.values().map(|(_, t)| t).sum();
            let percentage = if total > 0 { completed as f64 / total as f64 * 100.0 } else { 0.0 };

            if (percentage - last_percentage).abs() >= 0.5 || total == 0 {
                last_percentage = percentage;
                app.emit(
                    "model-pull-progress",
                    PullProgress { name: name.clone(), status, completed, total, percentage },
                )
                .ok();
            }
        }
    }

    Err(OllamaError::Api("the download ended unexpectedly".to_string()))
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

    #[test]
    fn normalizes_model_names() {
        let cases = [
            ("llama3.2:3b", "llama3.2:3b"),
            ("  qwen2.5  ", "qwen2.5"),
            ("https://ollama.com/library/qwen2.5:7b", "qwen2.5:7b"),
            ("ollama.com/someone/model", "someone/model"),
            ("hf.co/bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M", "hf.co/bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M"),
            ("https://huggingface.co/user/repo", "hf.co/user/repo"),
        ];
        for (input, expected) in cases {
            assert_eq!(normalize_model_name(input).unwrap(), expected, "{input}");
        }
    }

    #[test]
    fn rejects_bad_model_names() {
        for bad in ["", "mod el", "a/../b", "model:", "a:b:c", "rm -rf", "x;y"] {
            assert!(normalize_model_name(bad).is_err(), "{bad} should be rejected");
        }
    }
}
