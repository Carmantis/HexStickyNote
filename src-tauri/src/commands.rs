//! Tauri IPC Commands
//!
//! These commands are exposed to the frontend via the invoke() function.

use crate::assistant::{self, tools::ToolContext, AssistantContext, AssistantTurn, Decision, OllamaBackend};
use crate::calendar::db::DbPool;
use crate::card_manager::{self, Card};
use crate::claude_mcp;
use crate::hextime::HexTime;
use crate::http::HttpClient;
use crate::ollama::{self, ModelInfo};
use crate::settings_manager::SettingsManager;
use crate::window_state::{WindowState};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelList {
    pub models: Vec<ModelInfo>,
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

/// List the models installed in Ollama
#[tauri::command]
pub async fn list_local_models(http: State<'_, HttpClient>) -> Result<LocalModelList, String> {
    match ollama::list_models(http.client()).await {
        Ok(models) => Ok(LocalModelList { models, ollama_available: true }),
        Err(e) => {
            log::info!("Ollama models not listed: {}", e);
            Ok(LocalModelList { models: Vec::new(), ollama_available: false })
        }
    }
}

/// Get the selected model id
#[tauri::command]
pub async fn get_active_model(
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<Option<String>, String> {
    Ok(settings.get_active_model())
}

/// Select the Ollama model the assistant uses (None clears the selection)
#[tauri::command]
pub async fn set_active_model(
    model_id: Option<String>,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
) -> Result<(), String> {
    if let Some(id) = &model_id {
        if !id.starts_with(ollama::OLLAMA_PREFIX) {
            return Err(format!("Invalid model id: {}", id));
        }
    }
    settings.set_active_model(model_id).map_err(|e| e.to_string())
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
// Model Download Commands
// ============================================================================

/// Pull a model from the Ollama library into Ollama
/// Progress is emitted as 'model-pull-progress' events; returns the model id
#[tauri::command]
pub async fn pull_model(
    name: String,
    app: tauri::AppHandle,
    http: State<'_, HttpClient>,
) -> Result<String, String> {
    ollama::pull_model(&app, http.client(), &name).await.map_err(|e| e.to_string())
}

/// Cancel the running model pull
#[tauri::command]
pub async fn cancel_model_pull() -> Result<(), String> {
    ollama::cancel_pull();
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
// Assistant Commands
// ============================================================================

/// Send the conversation (ending with the user's new message) to the assistant
#[tauri::command]
pub async fn assistant_send(
    messages: Vec<serde_json::Value>,
    context: Option<AssistantContext>,
    app: tauri::AppHandle,
    http: State<'_, HttpClient>,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
    hextime: State<'_, HexTime>,
) -> Result<AssistantTurn, String> {
    let backend = OllamaBackend::new(http.client(), &settings).await.map_err(|e| e.to_string())?;
    let calendar = app.try_state::<DbPool>();
    let ctx = ToolContext {
        calendar: calendar.as_ref().map(|db| db.inner()),
        hextime: &hextime,
        client: http.client(),
    };
    assistant::send(&backend, &ctx, &context.unwrap_or_default(), messages).await.map_err(|e| e.to_string())
}

/// Apply the user's decisions on pending assistant actions and continue
#[tauri::command]
pub async fn assistant_confirm(
    messages: Vec<serde_json::Value>,
    context: Option<AssistantContext>,
    decisions: Vec<Decision>,
    app: tauri::AppHandle,
    http: State<'_, HttpClient>,
    settings: State<'_, std::sync::Arc<SettingsManager>>,
    hextime: State<'_, HexTime>,
) -> Result<AssistantTurn, String> {
    let backend = OllamaBackend::new(http.client(), &settings).await.map_err(|e| e.to_string())?;
    let calendar = app.try_state::<DbPool>();
    let ctx = ToolContext {
        calendar: calendar.as_ref().map(|db| db.inner()),
        hextime: &hextime,
        client: http.client(),
    };
    assistant::confirm(&backend, &ctx, &context.unwrap_or_default(), messages, decisions).await.map_err(|e| e.to_string())
}

// ============================================================================
// HexTime Commands
// ============================================================================

/// Start the HexTime sidecar if needed and return the URL of its UI
#[tauri::command]
pub async fn hextime_start(
    hextime: State<'_, HexTime>,
    http: State<'_, HttpClient>,
) -> Result<String, String> {
    hextime
        .start(http.client())
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
