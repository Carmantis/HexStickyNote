//! Card Manager - Handles CRUD operations for cards
//!
//! Shared logic for both UI commands and AI tools.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

// In-memory storage (can be upgraded to SQLite later)
static CARDS: Lazy<Mutex<Vec<Card>>> = Lazy::new(|| Mutex::new(Vec::new()));

// ============================================================================
// Public API
// ============================================================================

/// Create a new card
pub fn create_card(content: String) -> Result<Card, String> {
    let now = chrono::Utc::now().timestamp();
    let card = Card {
        id: Uuid::new_v4().to_string(),
        content,
        created_at: now,
        updated_at: now,
    };

    CARDS
        .lock()
        .map_err(|e| e.to_string())?
        .push(card.clone());

    Ok(card)
}

/// Get all cards
pub fn get_all_cards() -> Result<Vec<Card>, String> {
    let cards = CARDS.lock().map_err(|e| e.to_string())?.clone();
    Ok(cards)
}

/// Update a card
pub fn update_card(id: &str, content: Option<String>) -> Result<Card, String> {
    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;

    if let Some(existing) = cards.iter_mut().find(|c| c.id == id) {
        if let Some(c) = content {
            existing.content = c;
        }
        existing.updated_at = chrono::Utc::now().timestamp();
        Ok(existing.clone())
    } else {
        Err(format!("Card with id {} not found", id))
    }
}

/// Delete a card
pub fn delete_card(id: &str) -> Result<(), String> {
    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;
    let initial_len = cards.len();
    cards.retain(|c| c.id != id);
    
    if cards.len() == initial_len {
        return Err(format!("Card with id {} not found", id));
    }
    
    Ok(())
}
