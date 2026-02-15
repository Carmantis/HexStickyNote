//! Secure API key storage using Windows Credential Locker
//!
//! This module provides secure storage for API keys using the OS-level
//! credential manager. Keys are NEVER stored in plaintext files.

use keyring::Entry;
use thiserror::Error;

const SERVICE_NAME: &str = "HexStickyNote";

#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("Failed to access credential store: {0}")]
    AccessError(String),
    #[error("Key not found for provider: {0}")]
    KeyNotFound(String),
    #[error("Invalid provider: {0}")]
    InvalidProvider(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    Google,
    Poro2_8B,
    FinChatSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuType {
    Cpu,
    Vulkan,
    Cuda,
    Rocm,
}

impl GpuType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GpuType::Cpu => "cpu",
            GpuType::Vulkan => "vulkan",
            GpuType::Cuda => "cuda",
            GpuType::Rocm => "rocm",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vulkan" => GpuType::Vulkan,
            "cuda" => GpuType::Cuda,
            "rocm" => GpuType::Rocm,
            _ => GpuType::Cpu,
        }
    }
}

impl AiProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiProvider::OpenAI => "openai",
            AiProvider::Anthropic => "anthropic",
            AiProvider::Google => "google",
            AiProvider::Poro2_8B => "poro2_8b",
            AiProvider::FinChatSummary => "finchat_summary",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::OpenAI => "OpenAI",
            AiProvider::Anthropic => "Anthropic",
            AiProvider::Google => "Google",
            AiProvider::Poro2_8B => "Poro 2 8B Instruct",
            AiProvider::FinChatSummary => "FIN Chat Summarization",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, KeyringError> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(AiProvider::OpenAI),
            "anthropic" => Ok(AiProvider::Anthropic),
            "google" => Ok(AiProvider::Google),
            "poro2_8b" => Ok(AiProvider::Poro2_8B),
            "finchat_summary" => Ok(AiProvider::FinChatSummary),
            _ => Err(KeyringError::InvalidProvider(s.to_string())),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            AiProvider::OpenAI,
            AiProvider::Anthropic,
            AiProvider::Google,
            AiProvider::Poro2_8B,
            AiProvider::FinChatSummary,
        ]
    }

    /// Returns true if this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        match self {
            AiProvider::OpenAI | AiProvider::Anthropic | AiProvider::Google => true,
            AiProvider::Poro2_8B | AiProvider::FinChatSummary => false,
        }
    }
}

/// Keyring-based secure credential store
pub struct KeyringStore;

impl KeyringStore {
    /// Save an API key securely to the OS credential store
    pub fn save_api_key(provider: AiProvider, api_key: &str) -> Result<(), KeyringError> {
        let entry = Self::get_entry(provider)?;

        entry
            .set_password(api_key)
            .map_err(|e| KeyringError::AccessError(e.to_string()))?;

        log::info!("API key saved securely for provider: {}", provider.as_str());
        Ok(())
    }

    /// Retrieve an API key from the OS credential store
    pub fn get_api_key(provider: AiProvider) -> Result<String, KeyringError> {
        let entry = Self::get_entry(provider)?;

        entry
            .get_password()
            .map_err(|e| match e {
                keyring::Error::NoEntry => KeyringError::KeyNotFound(provider.as_str().to_string()),
                _ => KeyringError::AccessError(e.to_string()),
            })
    }

    /// Delete an API key from the OS credential store
    pub fn delete_api_key(provider: AiProvider) -> Result<(), KeyringError> {
        let entry = Self::get_entry(provider)?;

        entry
            .delete_credential()
            .map_err(|e| KeyringError::AccessError(e.to_string()))?;

        log::info!("API key deleted for provider: {}", provider.as_str());
        Ok(())
    }

    /// Check if an API key exists for a provider (or if local model is available)
    pub fn has_api_key(provider: AiProvider) -> bool {
        // Local providers don't need API keys, check model availability instead
        if !provider.requires_api_key() {
            return true; // We'll check model files separately in local_model module
        }
        Self::get_api_key(provider).is_ok()
    }

    /// Get list of providers with configured API keys (or available local models)
    pub fn get_configured_providers() -> Vec<AiProvider> {
        AiProvider::all()
            .into_iter()
            .filter(|p| {
                if p.requires_api_key() {
                    Self::has_api_key(*p)
                } else {
                    // Local models are always "configured" (UI will handle download state)
                    true
                }
            })
            .collect()
    }

    fn get_entry(provider: AiProvider) -> Result<Entry, KeyringError> {
        let username = format!("api_key_{}", provider.as_str());

        Entry::new(SERVICE_NAME, &username)
            .map_err(|e| KeyringError::AccessError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_roundtrip() {
        for provider in AiProvider::all() {
            let s = provider.as_str();
            let parsed = AiProvider::from_str(s).unwrap();
            assert_eq!(provider, parsed);
        }
    }
}
