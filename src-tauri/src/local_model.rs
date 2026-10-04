//! Local Model Management
//!
//! Downloads GGUF models from an Ollama-compatible registry (ollama.com library,
//! user namespaces, or hf.co) into the app's own models directory, and lists,
//! resolves and deletes those downloaded models.
//!
//! Model ids:
//! - `app:<file>.gguf`  – downloaded by HexStickyNote, run with the built-in llama.cpp
//! - `ollama:<name>`    – installed in a local Ollama, run through its API (see ollama.rs)

use directories::ProjectDirs;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

pub const APP_PREFIX: &str = "app:";
pub const OLLAMA_PREFIX: &str = "ollama:";

const DEFAULT_REGISTRY: &str = "registry.ollama.ai";
const MODEL_LAYER: &str = "application/vnd.ollama.image.model";
const MANIFEST_ACCEPT: &str = "application/vnd.docker.distribution.manifest.v2+json";

static DOWNLOAD_ACTIVE: AtomicBool = AtomicBool::new(false);
static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Error)]
pub enum LocalModelError {
    #[error("Failed to determine model directory: {0}")]
    DirectoryError(String),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid model name: {0}")]
    InvalidName(String),
    #[error("Model not found in registry: {0}")]
    NotFound(String),
    #[error("Unexpected registry response: {0}")]
    ManifestError(String),
    #[error("{0} has no GGUF model layer")]
    NoModelLayer(String),
    #[error("Another model download is already in progress")]
    DownloadInProgress,
    #[error("Download cancelled")]
    Cancelled,
    #[error("Downloaded file is corrupted (checksum mismatch)")]
    ChecksumMismatch,
    #[error("Model is not downloaded: {0}")]
    ModelMissing(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelSource {
    App,
    Ollama,
}

/// A model that can be selected for AI writing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelInfo {
    pub id: String,
    pub name: String,
    pub source: ModelSource,
    pub size: Option<u64>,
    /// Short human-readable details, e.g. "27.3B · Q4_K_M"
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadProgress {
    pub name: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadComplete {
    pub name: String,
    pub id: String,
}

// ============================================================================
// Model names
// ============================================================================

/// A model reference in an Ollama-compatible registry, e.g. `llama3.2:3b`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub host: String,
    pub path: String,
    pub tag: String,
}

impl ModelRef {
    /// Parse what a user typed or pasted:
    /// `llama3.2`, `llama3.2:3b`, `user/model:tag`, `hf.co/user/repo:Q4_K_M`,
    /// or an ollama.com URL like `https://ollama.com/library/llama3.2:3b`.
    pub fn parse(input: &str) -> Result<Self, LocalModelError> {
        let invalid = || LocalModelError::InvalidName(input.trim().to_string());

        let mut rest = input.trim();
        for prefix in ["https://", "http://"] {
            rest = rest.strip_prefix(prefix).unwrap_or(rest);
        }
        for prefix in ["www.ollama.com/", "ollama.com/", "registry.ollama.ai/"] {
            rest = rest.strip_prefix(prefix).unwrap_or(rest);
        }
        let rest = rest.trim_end_matches('/');

        let (name, tag) = match rest.rfind(':') {
            Some(i) if i > rest.rfind('/').map_or(0, |s| s + 1) => (&rest[..i], &rest[i + 1..]),
            _ => (rest, "latest"),
        };

        let mut segments: Vec<&str> = name.split('/').collect();
        let host = if segments.len() >= 3 && segments[0].contains('.') {
            match segments.remove(0) {
                "huggingface.co" => "hf.co".to_string(),
                h => h.to_lowercase(),
            }
        } else {
            DEFAULT_REGISTRY.to_string()
        };

        if segments.len() == 1 {
            segments.insert(0, "library");
        }

        let valid = |s: &str| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
        };
        if segments.len() != 2 || !segments.iter().all(|s| valid(s)) || !valid(tag) {
            return Err(invalid());
        }

        Ok(Self {
            host,
            path: segments.join("/"),
            tag: tag.to_string(),
        })
    }

    /// Short display name, e.g. `llama3.2:3b` or `hf.co/user/repo:Q4_K_M`
    pub fn display_name(&self) -> String {
        let path = self.path.strip_prefix("library/").unwrap_or(&self.path);
        if self.host == DEFAULT_REGISTRY {
            format!("{}:{}", path, self.tag)
        } else {
            format!("{}/{}:{}", self.host, path, self.tag)
        }
    }

    /// File name the model is stored under in the app's models directory
    pub fn file_name(&self) -> String {
        format!("{}.gguf", self.display_name().replace(['/', ':'], "-"))
    }

    fn manifest_url(&self) -> String {
        format!("https://{}/v2/{}/manifests/{}", self.host, self.path, self.tag)
    }

    fn blob_url(&self, digest: &str) -> String {
        format!("https://{}/v2/{}/blobs/{}", self.host, self.path, digest)
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    layers: Vec<ManifestLayer>,
}

#[derive(Debug, Deserialize)]
struct ManifestLayer {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
}

// ============================================================================
// Downloaded (app) models
// ============================================================================

/// Get the directory where downloaded models are stored
pub fn get_models_dir() -> Result<PathBuf, LocalModelError> {
    let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote").ok_or_else(|| {
        LocalModelError::DirectoryError("Failed to determine project directories".to_string())
    })?;

    let models_dir = proj_dirs.data_dir().join("models");
    fs::create_dir_all(&models_dir)?;

    Ok(models_dir)
}

/// List GGUF models downloaded by the app
pub fn list_app_models() -> Result<Vec<LocalModelInfo>, LocalModelError> {
    let mut models = Vec::new();

    for entry in fs::read_dir(get_models_dir()?)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("gguf") {
            continue;
        }
        let (Some(file_name), Some(stem)) = (
            path.file_name().and_then(|n| n.to_str()),
            path.file_stem().and_then(|n| n.to_str()),
        ) else {
            continue;
        };

        models.push(LocalModelInfo {
            id: format!("{}{}", APP_PREFIX, file_name),
            name: stem.to_string(),
            source: ModelSource::App,
            size: fs::metadata(&path).ok().map(|m| m.len()),
            details: None,
        });
    }

    models.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(models)
}

/// Resolve an `app:` model id to its file, rejecting anything outside the models directory
pub fn app_model_path(id: &str) -> Result<PathBuf, LocalModelError> {
    let file_name = id
        .strip_prefix(APP_PREFIX)
        .filter(|f| !f.is_empty() && !f.contains(['/', '\\']) && *f != "..")
        .ok_or_else(|| LocalModelError::InvalidName(id.to_string()))?;

    let path = get_models_dir()?.join(file_name);
    if !path.is_file() {
        return Err(LocalModelError::ModelMissing(file_name.to_string()));
    }
    Ok(path)
}

/// Delete a model downloaded by the app
pub fn delete_app_model(id: &str) -> Result<(), LocalModelError> {
    let path = app_model_path(id)?;
    fs::remove_file(&path)?;
    log::info!("Model deleted: {:?}", path);
    Ok(())
}

// ============================================================================
// Download
// ============================================================================

/// Resets the "download in progress" flag when the download ends in any way
struct DownloadGuard;

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        DOWNLOAD_ACTIVE.store(false, Ordering::SeqCst);
    }
}

/// Ask the running download to stop
pub fn cancel_download() {
    DOWNLOAD_CANCELLED.store(true, Ordering::SeqCst);
}

/// Download a model from an Ollama-compatible registry.
/// Emits 'local-model-download-progress' and 'local-model-download-complete' events.
/// Returns the new model id.
pub async fn download_model(
    app: &AppHandle,
    client: &Client,
    input: &str,
) -> Result<String, LocalModelError> {
    let model_ref = ModelRef::parse(input)?;
    let name = model_ref.display_name();

    if DOWNLOAD_ACTIVE.swap(true, Ordering::SeqCst) {
        return Err(LocalModelError::DownloadInProgress);
    }
    let _guard = DownloadGuard;
    DOWNLOAD_CANCELLED.store(false, Ordering::SeqCst);

    let model_path = get_models_dir()?.join(model_ref.file_name());
    let id = format!("{}{}", APP_PREFIX, model_ref.file_name());

    if model_path.exists() {
        log::info!("Model already downloaded: {:?}", model_path);
        app.emit("local-model-download-complete", ModelDownloadComplete { name, id: id.clone() })
            .ok();
        return Ok(id);
    }

    // 1. Manifest -> GGUF layer
    log::info!("Fetching manifest: {}", model_ref.manifest_url());
    let response = client
        .get(model_ref.manifest_url())
        .header("Accept", MANIFEST_ACCEPT)
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(LocalModelError::NotFound(name));
    }
    let manifest: Manifest = response
        .error_for_status()?
        .json()
        .await
        .map_err(|e| LocalModelError::ManifestError(e.to_string()))?;

    let layer = manifest
        .layers
        .into_iter()
        .find(|l| l.media_type == MODEL_LAYER)
        .ok_or_else(|| LocalModelError::NoModelLayer(name.clone()))?;

    let expected_hash = layer
        .digest
        .strip_prefix("sha256:")
        .ok_or_else(|| LocalModelError::ManifestError(format!("unsupported digest {}", layer.digest)))?
        .to_lowercase();

    // 2. Stream the blob to a temporary file while hashing it
    log::info!("Downloading {} ({} bytes)", name, layer.size);
    let response = client
        .get(model_ref.blob_url(&layer.digest))
        .send()
        .await?
        .error_for_status()?;

    let total_size = Some(response.content_length().unwrap_or(layer.size));
    let temp_path = model_path.with_extension("gguf.partial");
    let mut file = tokio::fs::File::create(&temp_path).await?;
    let mut stream = response.bytes_stream();
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_emitted_percentage = -1.0;

    let result: Result<(), LocalModelError> = async {
        while let Some(chunk) = stream.next().await {
            if DOWNLOAD_CANCELLED.load(Ordering::SeqCst) {
                return Err(LocalModelError::Cancelled);
            }

            let chunk = chunk?;
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;

            let percentage = total_size
                .filter(|t| *t > 0)
                .map(|t| downloaded as f64 / t as f64 * 100.0)
                .unwrap_or(0.0);

            if (percentage - last_emitted_percentage).abs() >= 0.5 {
                last_emitted_percentage = percentage;
                app.emit(
                    "local-model-download-progress",
                    ModelDownloadProgress {
                        name: name.clone(),
                        bytes_downloaded: downloaded,
                        total_bytes: total_size,
                        percentage,
                    },
                )
                .ok();
            }
        }
        file.flush().await?;
        Ok(())
    }
    .await;

    drop(file);

    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(e);
    }

    let actual_hash: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    if actual_hash != expected_hash {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(LocalModelError::ChecksumMismatch);
    }

    tokio::fs::rename(&temp_path, &model_path).await?;
    log::info!("Model downloaded successfully: {:?}", model_path);

    app.emit("local-model-download-complete", ModelDownloadComplete { name, id: id.clone() })
        .ok();

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(host: &str, path: &str, tag: &str) -> ModelRef {
        ModelRef { host: host.into(), path: path.into(), tag: tag.into() }
    }

    #[test]
    fn parses_model_names() {
        let ollama = DEFAULT_REGISTRY;
        assert_eq!(ModelRef::parse("llama3.2").unwrap(), r(ollama, "library/llama3.2", "latest"));
        assert_eq!(ModelRef::parse(" llama3.2:3b ").unwrap(), r(ollama, "library/llama3.2", "3b"));
        assert_eq!(ModelRef::parse("user/my-model:q4").unwrap(), r(ollama, "user/my-model", "q4"));
        assert_eq!(
            ModelRef::parse("https://ollama.com/library/qwen2.5:7b").unwrap(),
            r(ollama, "library/qwen2.5", "7b")
        );
        assert_eq!(
            ModelRef::parse("hf.co/bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M").unwrap(),
            r("hf.co", "bartowski/Llama-3.2-3B-Instruct-GGUF", "Q4_K_M")
        );
        assert_eq!(
            ModelRef::parse("https://huggingface.co/user/repo").unwrap(),
            r("hf.co", "user/repo", "latest")
        );
    }

    #[test]
    fn rejects_bad_names() {
        for bad in ["", "a/b/c", "../etc", "model:", "mod el", "a/..:x"] {
            assert!(ModelRef::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn builds_display_and_file_names() {
        let m = ModelRef::parse("llama3.2:3b").unwrap();
        assert_eq!(m.display_name(), "llama3.2:3b");
        assert_eq!(m.file_name(), "llama3.2-3b.gguf");

        let h = ModelRef::parse("hf.co/user/repo:Q4_K_M").unwrap();
        assert_eq!(h.display_name(), "hf.co/user/repo:Q4_K_M");
        assert_eq!(h.file_name(), "hf.co-user-repo-Q4_K_M.gguf");
    }

    #[test]
    fn rejects_path_traversal_ids() {
        for bad in ["app:../x.gguf", "app:a/b.gguf", "app:", "ollama:x", "x.gguf"] {
            assert!(matches!(app_model_path(bad), Err(LocalModelError::InvalidName(_))), "{bad}");
        }
    }
}
