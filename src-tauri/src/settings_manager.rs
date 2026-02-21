//! Application Settings Manager
//!
//! Manages user preferences including AI model selections and local model configurations.
//! Settings are stored in a JSON file separate from API keys (which use keyring).

use crate::keyring_store::{AiProvider, GpuType};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("Failed to determine settings directory: {0}")]
    DirectoryError(String),
    #[error("Failed to read settings file: {0}")]
    ReadError(String),
    #[error("Failed to write settings file: {0}")]
    WriteError(String),
    #[error("Failed to parse settings: {0}")]
    ParseError(String),
}

/// Configuration for a cloud AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// The model to use (e.g., "gpt-4o", "claude-3-5-sonnet-20241022")
    pub model: String,
    /// Custom model name if user wants to use a different model
    pub custom_model: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            custom_model: None,
        }
    }
}

/// Configuration for a local model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelConfig {
    /// HuggingFace repository (e.g., "mradermacher/Llama-Poro-2-8B-Instruct-GGUF")
    pub repo: String,
    /// GGUF filename in the repo
    pub filename: String,
    /// Custom download URL (overrides repo/filename if set)
    pub custom_url: Option<String>,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            repo: String::new(),
            filename: String::new(),
            custom_url: None,
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Cloud provider configurations (openai, anthropic, google)
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// Local model configurations (poro2_8b, llama3_8b)
    #[serde(default)]
    pub local_models: HashMap<String, LocalModelConfig>,
    /// GPU acceleration type (cpu, vulkan, cuda, rocm)
    #[serde(default = "default_gpu_type")]
    pub gpu_type: GpuType,
}

fn default_gpu_type() -> GpuType {
    GpuType::Cpu
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut providers = HashMap::new();
        let mut local_models = HashMap::new();

        // Default cloud provider models
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                model: "gpt-5.2-codex".to_string(),
                custom_model: None,
            },
        );
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                model: "claude-sonnet-4-6".to_string(),
                custom_model: None,
            },
        );
        providers.insert(
            "google".to_string(),
            ProviderConfig {
                model: "gemini-3.1-pro-latest".to_string(),
                custom_model: None,
            },
        );

        // Default local models
        local_models.insert(
            "poro2_8b".to_string(),
            LocalModelConfig {
                repo: "mradermacher/Llama-Poro-2-8B-Instruct-GGUF".to_string(),
                filename: "Llama-Poro-2-8B-Instruct.Q4_K_M.gguf".to_string(),
                custom_url: None,
            },
        );
        local_models.insert(
            "llama3_8b".to_string(),
            LocalModelConfig {
                repo: "mradermacher/Meta-Llama-3.1-8B-Instruct-GGUF".to_string(),
                filename: "Meta-Llama-3.1-8B-Instruct.Q4_K_M.gguf".to_string(),
                custom_url: None,
            },
        );

        Self {
            providers,
            local_models,
            gpu_type: GpuType::Cpu,
        }
    }
}

/// Global settings manager with thread-safe access
pub struct SettingsManager {
    settings: RwLock<AppSettings>,
    settings_path: PathBuf,
}

impl SettingsManager {
    /// Create a new settings manager
    pub fn new() -> Result<Self, SettingsError> {
        let settings_path = Self::get_settings_path()?;
        let settings = Self::load_from_disk(&settings_path)?;

        Ok(Self {
            settings: RwLock::new(settings),
            settings_path,
        })
    }

    /// Get the path to the settings file
    fn get_settings_path() -> Result<PathBuf, SettingsError> {
        let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
            .ok_or_else(|| {
                SettingsError::DirectoryError("Failed to determine project directories".to_string())
            })?;

        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).map_err(|e| {
            SettingsError::DirectoryError(format!("Failed to create config directory: {}", e))
        })?;

        Ok(config_dir.join("settings.json"))
    }

    /// Load settings from disk, creating defaults if file doesn't exist
    fn load_from_disk(path: &PathBuf) -> Result<AppSettings, SettingsError> {
        if !path.exists() {
            log::info!("Settings file not found, creating defaults");
            let defaults = AppSettings::default();
            Self::save_to_disk(path, &defaults)?;
            return Ok(defaults);
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| SettingsError::ReadError(format!("Failed to read settings: {}", e)))?;

        match serde_json::from_str(&contents) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                log::warn!("Failed to parse settings, using defaults: {}", e);
                Ok(AppSettings::default())
            }
        }
    }

    /// Save settings to disk
    fn save_to_disk(path: &PathBuf, settings: &AppSettings) -> Result<(), SettingsError> {
        let json = serde_json::to_string_pretty(settings).map_err(|e| {
            SettingsError::WriteError(format!("Failed to serialize settings: {}", e))
        })?;

        fs::write(path, json).map_err(|e| {
            SettingsError::WriteError(format!("Failed to write settings: {}", e))
        })?;

        log::debug!("Settings saved to {:?}", path);
        Ok(())
    }

    /// Save current settings to disk
    fn save(&self) -> Result<(), SettingsError> {
        let settings = self.settings.read().unwrap();
        Self::save_to_disk(&self.settings_path, &*settings)
    }

    /// Get the model name for a cloud provider
    pub fn get_provider_model(&self, provider: AiProvider) -> String {
        let settings = self.settings.read().unwrap();
        let provider_key = provider.as_str();

        if let Some(config) = settings.providers.get(provider_key) {
            // Use custom model if set, otherwise use default
            config.custom_model.clone().unwrap_or_else(|| config.model.clone())
        } else {
            // Fallback to hardcoded defaults if not in settings
            match provider {
                AiProvider::OpenAI => "gpt-5.2-codex".to_string(),
                AiProvider::Anthropic => "claude-sonnet-4-6".to_string(),
                AiProvider::Google => "gemini-3.1-pro-latest".to_string(),
                _ => "unknown".to_string(),
            }
        }
    }

    /// Set the model for a cloud provider
    pub fn set_provider_model(
        &self,
        provider: AiProvider,
        model: String,
        is_custom: bool,
    ) -> Result<(), SettingsError> {
        let mut settings = self.settings.write().unwrap();
        let provider_key = provider.as_str().to_string();

        let config = settings
            .providers
            .entry(provider_key)
            .or_insert_with(ProviderConfig::default);

        if is_custom {
            config.custom_model = Some(model);
        } else {
            config.model = model;
            config.custom_model = None;
        }

        drop(settings);
        self.save()
    }

    /// Get local model configuration
    pub fn get_local_model_config(&self, provider: AiProvider) -> Option<LocalModelConfig> {
        let settings = self.settings.read().unwrap();
        settings.local_models.get(provider.as_str()).cloned()
    }

    /// Set local model configuration
    pub fn set_local_model_config(
        &self,
        provider: AiProvider,
        config: LocalModelConfig,
    ) -> Result<(), SettingsError> {
        let mut settings = self.settings.write().unwrap();
        settings
            .local_models
            .insert(provider.as_str().to_string(), config);
        drop(settings);
        self.save()
    }

    /// Get current GPU type
    pub fn get_gpu_type(&self) -> GpuType {
        let settings = self.settings.read().unwrap();
        settings.gpu_type
    }

    /// Set GPU type
    pub fn set_gpu_type(&self, gpu_type: GpuType) -> Result<(), SettingsError> {
        let mut settings = self.settings.write().unwrap();
        settings.gpu_type = gpu_type;
        drop(settings);
        self.save()
    }

    /// Get all settings (for frontend)
    pub fn get_all_settings(&self) -> AppSettings {
        self.settings.read().unwrap().clone()
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
