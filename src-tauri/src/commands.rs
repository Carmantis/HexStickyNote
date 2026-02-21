//! Tauri IPC Commands
//!
//! These commands are exposed to the frontend via the invoke() function.

use crate::ai_manager::AiManager;
use crate::card_manager::{self, Card};
use crate::claude_mcp;
use crate::keyring_store::{AiProvider, KeyringStore};
use crate::local_model::{self, ModelStatus};
use crate::settings_manager::SettingsManager;
use crate::window_state::{WindowState};
use serde::{Deserialize, Serialize};
use tauri::State;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub configured: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

// ============================================================================
// API Key Management Commands
// ============================================================================

/// Save an API key securely to the OS credential store
#[tauri::command]
pub async fn save_api_key(provider: String, key: String) -> Result<(), String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;

    KeyringStore::save_api_key(provider, &key).map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete an API key from the credential store
#[tauri::command]
pub async fn delete_api_key(provider: String) -> Result<(), String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;

    KeyringStore::delete_api_key(provider).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get list of all providers with their configuration status
#[tauri::command]
pub async fn get_providers() -> Vec<ProviderInfo> {
    AiProvider::all()
        .into_iter()
        .map(|p| ProviderInfo {
            id: p.as_str().to_string(),
            name: p.display_name().to_string(),
            configured: KeyringStore::has_api_key(p),
        })
        .collect()
}

/// Set the active AI provider
#[tauri::command]
pub async fn set_active_provider(
    provider: String,
    ai_manager: State<'_, AiManager>,
) -> Result<(), String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;

    if !KeyringStore::has_api_key(provider) {
        return Err(format!(
            "No API key configured for {}. Please add your API key in Settings.",
            provider.display_name()
        ));
    }

    ai_manager.set_active_provider(provider).await;

    Ok(())
}

/// Get the currently active provider
#[tauri::command]
pub async fn get_active_provider(ai_manager: State<'_, AiManager>) -> Result<Option<String>, String> {
    let provider = ai_manager.get_active_provider().await;
    Ok(provider.map(|p| p.as_str().to_string()))
}

// ============================================================================
// AI Streaming Commands
// ============================================================================

/// Invoke AI with streaming response
/// Results are emitted as 'ai-stream-chunk' events
#[tauri::command]
pub async fn invoke_ai_stream(
    prompt: String,
    context: String,
    app: tauri::AppHandle,
    ai_manager: State<'_, AiManager>,
) -> Result<(), String> {
    ai_manager
        .invoke_stream(&app, &prompt, &context)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Card Storage Commands (In-Memory for now, can be extended to SQLite)
// ============================================================================

/// Create a new card
#[tauri::command]
pub async fn create_card(content: String) -> Result<Card, String> {
    card_manager::create_card(content)
}

/// Get all cards
#[tauri::command]
pub async fn get_cards() -> Result<Vec<Card>, String> {
    card_manager::get_all_cards()
}

/// Update a card
#[tauri::command]
pub async fn save_card(card: Card) -> Result<(), String> {
    card_manager::update_card(&card.id, Some(card.content))?;
    Ok(())
}

/// Delete a card
#[tauri::command]
pub async fn delete_card(id: String) -> Result<(), String> {
    card_manager::delete_card(&id)
}

/// Reload all cards from file system
/// Useful when cards are modified externally (e.g., by Claude Desktop MCP)
#[tauri::command]
pub async fn reload_cards() -> Result<Vec<Card>, String> {
    card_manager::reload_all_cards()
}

// ============================================================================
// Window State Commands
// ============================================================================

/// Load window positions from disk
#[tauri::command]
pub async fn load_window_state() -> Result<WindowState, String> {
    WindowState::load()
}

/// Save main window position
#[tauri::command]
pub async fn save_main_window_position(x: i32, y: i32) -> Result<(), String> {
    let mut state = WindowState::load().unwrap_or_default();
    state.set_main_position(x, y);
    state.save()
}

/// Save orb window position
#[tauri::command]
pub async fn save_orb_window_position(x: i32, y: i32) -> Result<(), String> {
    let mut state = WindowState::load().unwrap_or_default();
    state.set_orb_position(x, y);
    state.save()
}

// ============================================================================
// Settings Commands
// ============================================================================

/// Get all application settings (model configurations, etc.)
#[tauri::command]
pub async fn get_all_settings(
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let app_settings = settings.get_all_settings();
    serde_json::to_value(app_settings).map_err(|e| e.to_string())
}

/// Set model for a cloud provider
#[tauri::command]
pub async fn set_provider_model(
    provider: String,
    model: String,
    is_custom: bool,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;
    settings
        .set_provider_model(provider, model, is_custom)
        .map_err(|e| e.to_string())
}

/// Set local model configuration
#[tauri::command]
pub async fn set_local_model_config(
    provider: String,
    repo: String,
    filename: String,
    custom_url: Option<String>,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    use crate::settings_manager::LocalModelConfig;

    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;
    let config = LocalModelConfig {
        repo,
        filename,
        custom_url,
    };
    settings
        .set_local_model_config(provider, config)
        .map_err(|e| e.to_string())
}

/// Set GPU acceleration type
#[tauri::command]
pub async fn set_gpu_type(
    gpu_type: String,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    use crate::keyring_store::GpuType;
    let gpu = GpuType::from_str(&gpu_type);
    settings.set_gpu_type(gpu).map_err(|e| e.to_string())
}

/// Get recommended models for each provider
#[tauri::command]
pub async fn get_recommended_models() -> Result<serde_json::Value, String> {
    let models = serde_json::json!({
        "openai": [
            { "id": "gpt-4o", "name": "GPT-4o (Recommended)" },
            { "id": "gpt-4o-mini", "name": "GPT-4o Mini (Faster, cheaper)" },
            { "id": "gpt-4-turbo", "name": "GPT-4 Turbo" },
            { "id": "gpt-3.5-turbo", "name": "GPT-3.5 Turbo (Legacy)" },
        ],
        "anthropic": [
            { "id": "claude-3-5-sonnet-20241022", "name": "Claude 3.5 Sonnet (Recommended)" },
            { "id": "claude-3-5-haiku-20241022", "name": "Claude 3.5 Haiku (Faster)" },
            { "id": "claude-3-opus-20240229", "name": "Claude 3 Opus (Most capable)" },
        ],
        "google": [
            { "id": "gemini-2.5-flash", "name": "Gemini 2.5 Flash (Recommended)" },
            { "id": "gemini-2.5-pro", "name": "Gemini 2.5 Pro (Most capable)" },
            { "id": "gemini-1.5-pro", "name": "Gemini 1.5 Pro (Legacy)" },
            { "id": "gemini-1.5-flash", "name": "Gemini 1.5 Flash (Legacy)" },
        ],
    });
    Ok(models)
}

// ============================================================================
// Local Model Commands
// ============================================================================

/// Get status of a local model (downloaded, file size, etc.)
#[tauri::command]
pub async fn get_local_model_status(
    provider: String,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<ModelStatus, String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;
    local_model::get_model_status(provider, Some(&settings)).map_err(|e| e.to_string())
}

/// Download a local model from HuggingFace
/// Progress is emitted as 'local-model-download-progress' events
/// Completion is emitted as 'local-model-download-complete' event
#[tauri::command]
pub async fn download_local_model(
    provider: String,
    app: tauri::AppHandle,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;
    local_model::download_model(&app, provider, Some(&settings))
        .await
        .map_err(|e| e.to_string())
}

/// Delete a downloaded local model
#[tauri::command]
pub async fn delete_local_model(
    provider: String,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    let provider = AiProvider::from_str(&provider).map_err(|e| e.to_string())?;
    local_model::delete_model(provider, Some(&settings))
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Application Control Commands
// ============================================================================

/// Exit the entire application (all windows)
#[tauri::command]
pub async fn exit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

// ============================================================================
// Claude Desktop MCP Commands
// ============================================================================

/// Check Claude Desktop MCP integration status
#[tauri::command]
pub async fn check_claude_mcp(app: tauri::AppHandle) -> Result<claude_mcp::ClaudeMcpStatus, String> {
    claude_mcp::check_status(&app)
}

/// Setup Claude Desktop MCP integration
#[tauri::command]
pub async fn setup_claude_mcp(app: tauri::AppHandle) -> Result<(), String> {
    claude_mcp::setup(&app)
}

/// Remove Claude Desktop MCP integration
#[tauri::command]
pub async fn remove_claude_mcp() -> Result<(), String> {
    claude_mcp::remove()
}

/// Open cards directory in file explorer
#[tauri::command]
pub async fn open_cards_directory() -> Result<(), String> {
    let cards_dir = card_manager::get_cards_directory()
        .map_err(|e| format!("Failed to get cards directory: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&cards_dir)
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&cards_dir)
            .spawn()
            .map_err(|e| format!("Failed to open finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&cards_dir)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    log::info!("Opened cards directory: {:?}", cards_dir);
    Ok(())
}
