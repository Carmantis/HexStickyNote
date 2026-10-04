//! Tauri IPC Commands
//!
//! These commands are exposed to the frontend via the invoke() function.

use crate::ai_manager::AiManager;
use crate::card_manager::{self, Card};
use crate::claude_mcp;
use crate::hextime::HexTime;
use crate::local_model::{self, LocalModelInfo};
use crate::ollama;
use crate::settings_manager::{GpuType, SettingsManager};
use crate::window_state::{WindowState};
use serde::{Deserialize, Serialize};
use tauri::State;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelList {
    pub models: Vec<LocalModelInfo>,
    /// Whether a local Ollama server answered
    pub ollama_available: bool,
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
// Model Selection Commands
// ============================================================================

/// List models downloaded by the app and models installed in Ollama
#[tauri::command]
pub async fn list_local_models(ai_manager: State<'_, AiManager>) -> Result<LocalModelList, String> {
    let mut models = local_model::list_app_models().map_err(|e| e.to_string())?;

    let ollama_available = match ollama::list_models(ai_manager.client()).await {
        Ok(ollama_models) => {
            models.extend(ollama_models);
            true
        }
        Err(e) => {
            log::info!("Ollama models not listed: {}", e);
            false
        }
    };

    Ok(LocalModelList { models, ollama_available })
}

/// Get the selected model id
#[tauri::command]
pub async fn get_active_model(
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<Option<String>, String> {
    Ok(settings.get_active_model())
}

/// Select the model used for AI writing (None clears the selection)
#[tauri::command]
pub async fn set_active_model(
    model_id: Option<String>,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    if let Some(id) = &model_id {
        if !id.starts_with(local_model::APP_PREFIX) && !id.starts_with(local_model::OLLAMA_PREFIX) {
            return Err(format!("Invalid model id: {}", id));
        }
    }
    settings.set_active_model(model_id).map_err(|e| e.to_string())
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

/// Set GPU acceleration type
#[tauri::command]
pub async fn set_gpu_type(
    gpu_type: String,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    let gpu = GpuType::from_str(&gpu_type);
    settings.set_gpu_type(gpu).map_err(|e| e.to_string())
}

// ============================================================================
// Local Model Commands
// ============================================================================

/// Download a model from the Ollama library (or another Ollama-compatible registry)
/// Progress is emitted as 'local-model-download-progress' events
/// Completion is emitted as 'local-model-download-complete' event
/// Returns the new model id
#[tauri::command]
pub async fn download_model(
    name: String,
    app: tauri::AppHandle,
    ai_manager: State<'_, AiManager>,
) -> Result<String, String> {
    local_model::download_model(&app, ai_manager.client(), &name)
        .await
        .map_err(|e| e.to_string())
}

/// Cancel the running model download
#[tauri::command]
pub async fn cancel_model_download() -> Result<(), String> {
    local_model::cancel_download();
    Ok(())
}

/// Delete a model downloaded by the app (Ollama models are managed by Ollama)
#[tauri::command]
pub async fn delete_local_model(
    model_id: String,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    local_model::delete_app_model(&model_id).map_err(|e| e.to_string())?;

    if settings.get_active_model().as_deref() == Some(model_id.as_str()) {
        settings.set_active_model(None).map_err(|e| e.to_string())?;
    }
    Ok(())
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

// ============================================================================
// HexTime Commands
// ============================================================================

/// Start the HexTime sidecar if needed and return the URL of its UI
#[tauri::command]
pub async fn hextime_start(
    hextime: State<'_, HexTime>,
    ai_manager: State<'_, AiManager>,
) -> Result<String, String> {
    hextime
        .start(ai_manager.client())
        .await
        .map_err(|e| e.to_string())
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
