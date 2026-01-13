//! Tauri IPC Commands
//!
//! These commands are exposed to the frontend via the invoke() function.

use crate::ai_manager::AiManager;
use crate::card_manager::{self, Card};
use crate::keyring_store::{AiProvider, KeyringStore};
use crate::window_state::{WindowPosition, WindowState};
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
// Application Control Commands
// ============================================================================

/// Exit the entire application (all windows)
#[tauri::command]
pub async fn exit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
