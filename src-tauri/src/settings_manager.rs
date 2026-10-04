//! Application Settings Manager
//!
//! Manages user preferences: the active local model and GPU acceleration.
//! Settings are stored in a JSON file.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
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

/// GPU acceleration type for the built-in llama.cpp backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuType {
    Cpu,
    Vulkan,
    Cuda,
    Rocm,
}

impl GpuType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vulkan" => GpuType::Vulkan,
            "cuda" => GpuType::Cuda,
            "rocm" => GpuType::Rocm,
            _ => GpuType::Cpu,
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Selected model id ("app:<file>.gguf" or "ollama:<name>"), see local_model.rs
    #[serde(default)]
    pub active_model: Option<String>,
    /// GPU acceleration type (cpu, vulkan, cuda, rocm)
    #[serde(default = "default_gpu_type")]
    pub gpu_type: GpuType,
}

fn default_gpu_type() -> GpuType {
    GpuType::Cpu
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_model: None,
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

    /// Get the selected model id
    pub fn get_active_model(&self) -> Option<String> {
        self.settings.read().unwrap().active_model.clone()
    }

    /// Set (or clear) the selected model id
    pub fn set_active_model(&self, model_id: Option<String>) -> Result<(), SettingsError> {
        let mut settings = self.settings.write().unwrap();
        settings.active_model = model_id;
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
