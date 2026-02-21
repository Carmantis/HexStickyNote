//! Local Model Management - Downloads and manages GGUF models from HuggingFace
//!
//! Handles downloading GGUF models for local inference.

use crate::keyring_store::AiProvider;
use crate::settings_manager::SettingsManager;
use directories::ProjectDirs;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LocalModelError {
    #[error("Failed to determine model directory: {0}")]
    DirectoryError(String),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid provider for local model: {0}")]
    InvalidProvider(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadProgress {
    pub provider: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadComplete {
    pub provider: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub provider: String,
    pub is_downloaded: bool,
    pub file_size: Option<u64>,
    pub path: Option<String>,
}

/// Get the directory where local models are stored
pub fn get_models_dir() -> Result<PathBuf, LocalModelError> {
    let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
        .ok_or_else(|| LocalModelError::DirectoryError("Failed to determine project directories".to_string()))?;

    let models_dir = proj_dirs.data_dir().join("models");
    fs::create_dir_all(&models_dir)?;

    Ok(models_dir)
}

/// Get the download URL and filename for a provider
fn get_model_info(
    provider: AiProvider,
    settings: Option<&SettingsManager>,
) -> Result<(String, String), LocalModelError> {
    // Check for custom URL in settings
    if let Some(settings_mgr) = settings {
        if let Some(config) = settings_mgr.get_local_model_config(provider) {
            if let Some(custom_url) = config.custom_url {
                // Extract filename from URL
                let filename = custom_url
                    .split('/')
                    .last()
                    .unwrap_or("model.gguf")
                    .to_string();
                return Ok((custom_url, filename));
            }
            // Use repo/filename from settings
            if !config.repo.is_empty() && !config.filename.is_empty() {
                let url = format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    config.repo, config.filename
                );
                return Ok((url, config.filename));
            }
        }
    }

    // Fallback to default models
    match provider {
        AiProvider::Poro2_8B => Ok((
            "https://huggingface.co/mradermacher/Llama-Poro-2-8B-Instruct-GGUF/resolve/main/Llama-Poro-2-8B-Instruct.Q4_K_M.gguf".to_string(),
            "Llama-Poro-2-8B-Instruct.Q4_K_M.gguf".to_string()
        )),
        AiProvider::Llama3_8B => Ok((
            "https://huggingface.co/mradermacher/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct.Q4_K_M.gguf".to_string(),
            "Meta-Llama-3.1-8B-Instruct.Q4_K_M.gguf".to_string()
        )),
        _ => Err(LocalModelError::InvalidProvider(format!("{:?} is not a local model provider", provider)))
    }
}

/// Get the path to a model file
pub fn get_model_path(
    provider: AiProvider,
    settings: Option<&SettingsManager>,
) -> Result<PathBuf, LocalModelError> {
    let (_, filename) = get_model_info(provider, settings)?;
    let models_dir = get_models_dir()?;
    Ok(models_dir.join(filename))
}

/// Check if a model is downloaded
pub fn is_model_downloaded(
    provider: AiProvider,
    settings: Option<&SettingsManager>,
) -> Result<bool, LocalModelError> {
    let model_path = get_model_path(provider, settings)?;
    Ok(model_path.exists())
}

/// Get model status
pub fn get_model_status(
    provider: AiProvider,
    settings: Option<&SettingsManager>,
) -> Result<ModelStatus, LocalModelError> {
    let model_path = get_model_path(provider, settings)?;
    let is_downloaded = model_path.exists();

    let (file_size, path) = if is_downloaded {
        let metadata = fs::metadata(&model_path)?;
        (Some(metadata.len()), Some(model_path.to_string_lossy().to_string()))
    } else {
        (None, None)
    };

    Ok(ModelStatus {
        provider: provider.as_str().to_string(),
        is_downloaded,
        file_size,
        path,
    })
}

/// Download a model from HuggingFace with progress tracking
pub async fn download_model(
    app: &AppHandle,
    provider: AiProvider,
    settings: Option<&SettingsManager>,
) -> Result<(), LocalModelError> {
    let (url, _filename) = get_model_info(provider, settings)?;
    let model_path = get_model_path(provider, settings)?;

    // Check if already downloaded
    if model_path.exists() {
        log::info!("Model already downloaded: {:?}", model_path);
        app.emit("local-model-download-complete", ModelDownloadComplete {
            provider: provider.as_str().to_string(),
            path: model_path.to_string_lossy().to_string(),
        }).ok();
        return Ok(());
    }

    log::info!("Downloading model from: {}", url);

    let client = Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(LocalModelError::HttpError(
            reqwest::Error::from(response.error_for_status().unwrap_err())
        ));
    }

    let total_size = response.content_length();

    // Create a temporary file
    let temp_path = model_path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&temp_path).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    let mut last_emitted_percentage = -1.0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;

        downloaded += chunk.len() as u64;

        let percentage = if let Some(total) = total_size {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // Emit progress event if percentage has changed by at least 0.5% or download is complete
        if (percentage - last_emitted_percentage).abs() >= 0.5 || downloaded == total_size.unwrap_or(0) {
            last_emitted_percentage = percentage;
            app.emit("local-model-download-progress", ModelDownloadProgress {
                provider: provider.as_str().to_string(),
                bytes_downloaded: downloaded,
                total_bytes: total_size,
                percentage,
            }).ok();
        }
    }

    // Ensure all data is written
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);

    // Rename temp file to final filename
    tokio::fs::rename(&temp_path, &model_path).await?;

    log::info!("Model downloaded successfully: {:?}", model_path);

    app.emit("local-model-download-complete", ModelDownloadComplete {
        provider: provider.as_str().to_string(),
        path: model_path.to_string_lossy().to_string(),
    }).ok();

    Ok(())
}

/// Delete a downloaded model
pub async fn delete_model(
    provider: AiProvider,
    settings: Option<&SettingsManager>,
) -> Result<(), LocalModelError> {
    let model_path = get_model_path(provider, settings)?;

    if model_path.exists() {
        tokio::fs::remove_file(&model_path).await?;
        log::info!("Model deleted: {:?}", model_path);
    }

    Ok(())
}
