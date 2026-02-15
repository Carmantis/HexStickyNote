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
pub fn get_cards_directory() -> Result<PathBuf, String> {
    let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
        .ok_or("Failed to determine project directories")?;

    let cards_dir = proj_dirs.data_dir().join("cards");
    fs::create_dir_all(&cards_dir)
        .map_err(|e| format!("Failed to create cards directory: {}", e))?;

    Ok(cards_dir)
}

/// Extract title from markdown content (first # heading or first meaningful line)
fn extract_title_from_content(content: &str) -> String {
    // 1. Look for first h1 (# Title)
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            let title = trimmed.trim_start_matches("# ").trim();
            if !title.is_empty() {
                return title.to_string();
            }
        } else if trimmed.starts_with('#') {
            // Handle #Title (no space)
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    // 2. Fallback: use first non-empty line that doesn't look like an AI command or metadata
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("---") {
            // Truncate long lines for title
            let mut title = trimmed.to_string();
            if title.len() > 50 {
                title.truncate(50);
                title.push_str("...");
            }
            return title;
        }
    }

    "Muistiinpano".to_string()
}

/// Sanitize title for use as filename
fn sanitize_filename(title: &str) -> String {
    // Remove or replace invalid Windows filename characters: \ / : * ? " < > |
    let mut sanitized = title
        .replace('\\', "-")
        .replace('/', "-")
        .replace(':', "-")
        .replace('*', "-")
        .replace('?', "")
        .replace('"', "'")
        .replace('<', "(")
        .replace('>', ")")
        .replace('|', "-");

    // Trim whitespace and dots from ends
    sanitized = sanitized.trim().trim_end_matches('.').to_string();

    // Limit length to 100 characters
    if sanitized.len() > 100 {
        sanitized.truncate(100);
        sanitized = sanitized.trim().to_string();
    }

    // Ensure not empty
    if sanitized.is_empty() {
        sanitized = "Untitled".to_string();
    }

    sanitized
}

/// Get unique filename, handling duplicates by adding (2), (3), etc.
fn get_unique_filename(cards_dir: &PathBuf, base_name: &str) -> String {
    let path = cards_dir.join(format!("{}.md", base_name));
    if !path.exists() {
        return format!("{}.md", base_name);
    }

    // File exists, add number suffix
    let mut counter = 2;
    loop {
        let numbered_name = format!("{} ({})", base_name, counter);
        let path = cards_dir.join(format!("{}.md", numbered_name));
        if !path.exists() {
            return format!("{}.md", numbered_name);
        }
        counter += 1;
        if counter > 1000 {
            // Safety limit
            return format!("{}.md", Uuid::new_v4());
        }
    }
}

/// Get the path for a specific card (by ID or by content for new cards)
fn get_card_file_path(id: &str) -> Result<PathBuf, String> {
    let cards_dir = get_cards_directory()?;

    // Try to find existing file with this ID in front matter
    let entries = fs::read_dir(&cards_dir)
        .map_err(|e| format!("Failed to read cards directory: {}", e))?;

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Try to read and parse the file
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok((metadata, _)) = parse_markdown_with_frontmatter(&content) {
                        if metadata.id == id {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    // File not found - this is an error since card should exist
    Err(format!("Card file not found for ID: {}", id))
}

/// Get the path for a new card based on its content
fn get_new_card_file_path(content: &str) -> Result<PathBuf, String> {
    let cards_dir = get_cards_directory()?;
    let title = extract_title_from_content(content);
    let sanitized = sanitize_filename(&title);
    let filename = get_unique_filename(&cards_dir, &sanitized);
    Ok(cards_dir.join(filename))
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
fn save_card_to_file(card: &Card) -> Result<PathBuf, String> {
    let content = create_markdown_with_frontmatter(card)?;

    // Try to find existing file, or create new one based on content
    let file_path = match get_card_file_path(&card.id) {
        Ok(path) => path,
        Err(_) => {
            // New card - generate filename from content
            get_new_card_file_path(&card.content)?
        }
    };

    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write card file: {}", e))?;

    log::debug!("Saved card {} to {:?}", card.id, file_path);
    Ok(file_path)
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
    let _ = save_card_to_file(&card)?;

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
        // Get old file path before updating content
        let old_path = get_card_file_path(id).ok();

        if let Some(c) = content {
            existing.content = c;
        }
        existing.updated_at = chrono::Utc::now().timestamp();
        let updated = existing.clone();

        // Save to markdown file
        // Note: save_card_to_file will find the OLD path if it exists
        // so we need to handle the rename manually if the title changed
        let current_path = if let Some(ref path) = old_path {
            // It exists, let's write to it first
            let file_content = create_markdown_with_frontmatter(&updated)?;
            fs::write(path, file_content).map_err(|e| e.to_string())?;
            path.clone()
        } else {
            save_card_to_file(&updated)?
        };

        // If title changed, rename the file
        if let Some(old_path) = old_path {
            let cards_dir = get_cards_directory()?;
            let new_title = extract_title_from_content(&updated.content);
            let sanitized = sanitize_filename(&new_title);
            let new_filename = get_unique_filename(&cards_dir, &sanitized);
            let new_path = cards_dir.join(new_filename);

            if old_path != new_path {
                fs::rename(&old_path, &new_path).map_err(|e| format!("Failed to rename file: {}", e))?;
                log::debug!("Renamed card file from {:?} to {:?}", old_path, new_path);
            }
        }

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
