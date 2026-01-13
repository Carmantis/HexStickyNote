//! Card Manager - Handles CRUD operations for cards
//!
//! Shared logic for both UI commands and AI tools.
//! Cards are stored as individual markdown files with YAML front matter.

use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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

// Persistent storage with markdown files
static CARDS: Lazy<Mutex<Vec<Card>>> = Lazy::new(|| {
    let cards = load_cards_from_files().unwrap_or_else(|e| {
        log::warn!("Failed to load cards from files: {}. Starting with empty list.", e);
        Vec::new()
    });
    Mutex::new(cards)
});

// ============================================================================
// File Storage Functions
// ============================================================================

/// Metadata stored in YAML front matter
#[derive(Debug, Serialize, Deserialize)]
struct CardMetadata {
    id: String,
    created_at: i64,
    updated_at: i64,
}

/// Get the directory where cards are stored
fn get_cards_directory() -> Result<PathBuf, String> {
    let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
        .ok_or("Failed to determine project directories")?;

    let cards_dir = proj_dirs.data_dir().join("cards");
    fs::create_dir_all(&cards_dir)
        .map_err(|e| format!("Failed to create cards directory: {}", e))?;

    Ok(cards_dir)
}

/// Get the path for a specific card
fn get_card_file_path(id: &str) -> Result<PathBuf, String> {
    let cards_dir = get_cards_directory()?;
    Ok(cards_dir.join(format!("{}.md", id)))
}

/// Parse YAML front matter and content from markdown file
fn parse_markdown_with_frontmatter(content: &str) -> Result<(CardMetadata, String), String> {
    // Check if file starts with ---
    if !content.starts_with("---\n") {
        return Err("File does not start with YAML front matter".to_string());
    }

    // Find the closing ---
    let rest = &content[4..]; // Skip first "---\n"
    if let Some(end_pos) = rest.find("\n---\n") {
        let yaml_str = &rest[..end_pos];
        let markdown_content = &rest[end_pos + 5..]; // Skip "\n---\n"

        let metadata: CardMetadata = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse YAML front matter: {}", e))?;

        Ok((metadata, markdown_content.to_string()))
    } else {
        Err("Could not find closing --- for YAML front matter".to_string())
    }
}

/// Create markdown file content with YAML front matter
fn create_markdown_with_frontmatter(card: &Card) -> Result<String, String> {
    let metadata = CardMetadata {
        id: card.id.clone(),
        created_at: card.created_at,
        updated_at: card.updated_at,
    };

    let yaml = serde_yaml::to_string(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    Ok(format!("---\n{}---\n{}", yaml, card.content))
}

/// Load all cards from markdown files
fn load_cards_from_files() -> Result<Vec<Card>, String> {
    let cards_dir = get_cards_directory()?;

    let mut cards = Vec::new();

    // Read all .md files in the directory
    let entries = fs::read_dir(&cards_dir)
        .map_err(|e| format!("Failed to read cards directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            match load_card_from_file(&path) {
                Ok(card) => cards.push(card),
                Err(e) => log::warn!("Failed to load card from {:?}: {}", path, e),
            }
        }
    }

    log::info!("Loaded {} cards from markdown files", cards.len());
    Ok(cards)
}

/// Load a single card from a markdown file
fn load_card_from_file(path: &PathBuf) -> Result<Card, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let (metadata, markdown_content) = parse_markdown_with_frontmatter(&content)?;

    Ok(Card {
        id: metadata.id,
        content: markdown_content,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    })
}

/// Save a single card to a markdown file
fn save_card_to_file(card: &Card) -> Result<(), String> {
    let file_path = get_card_file_path(&card.id)?;
    let content = create_markdown_with_frontmatter(card)?;

    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write card file: {}", e))?;

    log::debug!("Saved card {} to markdown file", card.id);
    Ok(())
}

/// Delete a card's markdown file
fn delete_card_file(id: &str) -> Result<(), String> {
    let file_path = get_card_file_path(id)?;

    if file_path.exists() {
        fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete card file: {}", e))?;
        log::debug!("Deleted card file for {}", id);
    }

    Ok(())
}

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

    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;
    cards.push(card.clone());

    // Save to markdown file
    save_card_to_file(&card)?;

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
        let updated = existing.clone();

        // Save to markdown file
        save_card_to_file(&updated)?;

        Ok(updated)
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

    // Delete markdown file
    delete_card_file(id)?;

    Ok(())
}
