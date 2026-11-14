//! Card Manager - Handles CRUD operations for cards
//!
//! Shared logic for both UI commands and AI tools.
//! Cards are stored as individual markdown files with YAML front matter.

use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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
    /// Name of the subdirectory the note is stored in; empty for the cards
    /// directory itself. Folders are one level deep.
    #[serde(default)]
    pub folder: String,
}

// Persistent storage with markdown files
static CARDS: Lazy<Mutex<Vec<Card>>> = Lazy::new(|| {
    let cards = get_cards_directory().and_then(|root| scan(&root)).unwrap_or_else(|e| {
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
pub fn extract_title_from_content(content: &str) -> String {
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
            if trimmed.chars().count() > 50 {
                return format!("{}...", trimmed.chars().take(50).collect::<String>());
            }
            return trimmed.to_string();
        }
    }

    "Note".to_string()
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
    if sanitized.chars().count() > 100 {
        sanitized = sanitized.chars().take(100).collect::<String>().trim().to_string();
    }

    // Ensure not empty
    if sanitized.is_empty() {
        sanitized = "Untitled".to_string();
    }

    sanitized
}

/// A folder name as it is used on disk, or an error when it cannot be one.
/// The empty name stands for the cards directory itself.
fn clean_folder_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let cleaned = sanitize_filename(trimmed);
    // Hidden directories are not listed as folders
    if trimmed.starts_with('.') || cleaned.starts_with('.') {
        return Err(format!("“{}” is not a valid folder name", trimmed));
    }
    Ok(cleaned)
}

fn folder_path(root: &Path, folder: &str) -> PathBuf {
    if folder.is_empty() {
        root.to_path_buf()
    } else {
        root.join(folder)
    }
}

/// `<base>.md` in `dir`, or `<base> (2).md` and so on when that is taken.
/// `keep` is the note's current file: its own name does not count as taken.
fn unique_path(dir: &Path, base_name: &str, keep: Option<&Path>) -> PathBuf {
    let free = |path: &Path| !path.exists() || keep == Some(path);

    let path = dir.join(format!("{}.md", base_name));
    if free(&path) {
        return path;
    }
    for counter in 2..=1000 {
        let path = dir.join(format!("{} ({}).md", base_name, counter));
        if free(&path) {
            return path;
        }
    }
    dir.join(format!("{}.md", Uuid::new_v4()))
}

fn is_markdown(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md")
}

/// Names of the folders (subdirectories) in the cards directory, sorted
fn folders_in(root: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(root).map_err(|e| format!("Failed to read cards directory: {}", e))?;

    let mut folders: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .collect();
    folders.sort_by_key(|name| name.to_lowercase());
    Ok(folders)
}

/// Markdown files directly in `dir`
fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| is_markdown(p)).collect())
        .unwrap_or_default()
}

/// Every note file with the folder it is in
fn note_files(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut files: Vec<(String, PathBuf)> =
        markdown_files(root).into_iter().map(|path| (String::new(), path)).collect();
    for folder in folders_in(root)? {
        for path in markdown_files(&root.join(&folder)) {
            files.push((folder.clone(), path));
        }
    }
    Ok(files)
}

/// The file of the note with this id
fn find_file(root: &Path, id: &str) -> Option<PathBuf> {
    note_files(root).ok()?.into_iter().map(|(_, path)| path).find(|path| {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| parse_markdown_with_frontmatter(&content).ok())
            .is_some_and(|(metadata, _)| metadata.id == id)
    })
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

/// Load all cards from the cards directory and its folders
fn scan(root: &Path) -> Result<Vec<Card>, String> {
    let mut cards = Vec::new();
    for (folder, path) in note_files(root)? {
        match load_card_from_file(&path, folder) {
            Ok(card) => cards.push(card),
            Err(e) => log::warn!("Failed to load card from {:?}: {}", path, e),
        }
    }

    log::info!("Loaded {} cards from markdown files", cards.len());
    Ok(cards)
}

/// Load a single card from a markdown file
fn load_card_from_file(path: &Path, folder: String) -> Result<Card, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let (metadata, markdown_content) = parse_markdown_with_frontmatter(&content)?;

    Ok(Card {
        id: metadata.id,
        content: markdown_content,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        folder,
    })
}

/// Write a card to `<folder>/<title>.md`. A note that already has a file
/// elsewhere (its title or folder changed) is moved.
fn write_card(root: &Path, card: &Card) -> Result<PathBuf, String> {
    let content = create_markdown_with_frontmatter(card)?;
    let dir = folder_path(root, &card.folder);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create folder: {}", e))?;

    let old_path = find_file(root, &card.id);
    let base_name = sanitize_filename(&extract_title_from_content(&card.content));
    let path = unique_path(&dir, &base_name, old_path.as_deref());

    fs::write(&path, content).map_err(|e| format!("Failed to write card file: {}", e))?;
    if let Some(old_path) = old_path.filter(|old| *old != path) {
        fs::remove_file(&old_path).map_err(|e| format!("Failed to remove old card file: {}", e))?;
        log::debug!("Moved card file from {:?} to {:?}", old_path, path);
    }

    Ok(path)
}

fn make_folder(root: &Path, name: &str) -> Result<String, String> {
    let name = clean_folder_name(name)?;
    if name.is_empty() {
        return Err("Folder name is empty".to_string());
    }
    fs::create_dir_all(root.join(&name)).map_err(|e| format!("Failed to create folder: {}", e))?;
    Ok(name)
}

fn move_folder(root: &Path, from: &str, to: &str) -> Result<String, String> {
    let to = clean_folder_name(to)?;
    if from.is_empty() || to.is_empty() {
        return Err("Folder name is empty".to_string());
    }
    let (old_path, new_path) = (root.join(from), root.join(&to));
    if !old_path.is_dir() {
        return Err(format!("Folder “{}” not found", from));
    }
    // A change of letter case only is the same directory on some file systems
    if new_path.exists() && from.to_lowercase() != to.to_lowercase() {
        return Err(format!("Folder “{}” already exists", to));
    }
    fs::rename(&old_path, &new_path).map_err(|e| format!("Failed to rename folder: {}", e))?;
    Ok(to)
}

/// Remove a folder. Its notes are kept: they move to the cards directory.
fn remove_folder(root: &Path, name: &str) -> Result<(), String> {
    let dir = root.join(name);
    if name.is_empty() || !folders_in(root)?.iter().any(|f| f == name) {
        return Err(format!("Folder “{}” not found", name));
    }

    for path in markdown_files(&dir) {
        let base_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
        fs::rename(&path, unique_path(root, &base_name, None))
            .map_err(|e| format!("Failed to move a note out of the folder: {}", e))?;
    }
    fs::remove_dir(&dir)
        .map_err(|e| format!("The folder still contains other files and was kept: {}", e))
}

// ============================================================================
// Public API
// ============================================================================

/// Create a new card in `folder` (empty for none)
pub fn create_card(content: String, folder: &str) -> Result<Card, String> {
    let now = chrono::Utc::now().timestamp();
    let card = Card {
        id: Uuid::new_v4().to_string(),
        content,
        created_at: now,
        updated_at: now,
        folder: clean_folder_name(folder)?,
    };

    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;
    write_card(&get_cards_directory()?, &card)?;
    cards.push(card.clone());

    Ok(card)
}

/// Get all cards
pub fn get_all_cards() -> Result<Vec<Card>, String> {
    let cards = CARDS.lock().map_err(|e| e.to_string())?.clone();
    Ok(cards)
}

/// Update a card's content. The file is renamed when the title changed.
pub fn update_card(id: &str, content: Option<String>) -> Result<Card, String> {
    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;
    let existing = cards
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("Card with id {} not found", id))?;

    if let Some(c) = content {
        existing.content = c;
    }
    existing.updated_at = chrono::Utc::now().timestamp();

    write_card(&get_cards_directory()?, existing)?;
    Ok(existing.clone())
}

/// Move a card to another folder (empty for none)
pub fn move_card(id: &str, folder: &str) -> Result<Card, String> {
    let folder = clean_folder_name(folder)?;
    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;
    let existing = cards
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("Card with id {} not found", id))?;

    let moved = Card { folder, ..existing.clone() };
    write_card(&get_cards_directory()?, &moved)?;
    *existing = moved.clone();
    Ok(moved)
}

/// Delete a card
pub fn delete_card(id: &str) -> Result<(), String> {
    let mut cards = CARDS.lock().map_err(|e| e.to_string())?;
    if !cards.iter().any(|c| c.id == id) {
        return Err(format!("Card with id {} not found", id));
    }

    if let Some(path) = find_file(&get_cards_directory()?, id) {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete card file: {}", e))?;
    }
    cards.retain(|c| c.id != id);

    Ok(())
}

/// Reload all cards from the file system
/// This is useful when cards are modified externally (e.g., by MCP server)
pub fn reload_all_cards() -> Result<Vec<Card>, String> {
    let cards = scan(&get_cards_directory()?)?;

    let mut cards_lock = CARDS.lock().map_err(|e| e.to_string())?;
    *cards_lock = cards.clone();

    log::info!("Reloaded {} cards from file system", cards.len());
    Ok(cards)
}

/// Folder names, sorted. Empty folders are included.
pub fn list_folders() -> Result<Vec<String>, String> {
    folders_in(&get_cards_directory()?)
}

/// Create a folder and return its name as stored
pub fn create_folder(name: &str) -> Result<String, String> {
    make_folder(&get_cards_directory()?, name)
}

/// Rename a folder and return its new name as stored
pub fn rename_folder(from: &str, to: &str) -> Result<String, String> {
    let name = move_folder(&get_cards_directory()?, from, to)?;
    reload_all_cards()?;
    Ok(name)
}

/// Delete a folder; its notes move out of it and are kept
pub fn delete_folder(name: &str) -> Result<(), String> {
    remove_folder(&get_cards_directory()?, name)?;
    reload_all_cards()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// An empty cards directory of its own for one test
    fn temp_root() -> PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "hex-cards-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn card(id: &str, content: &str, folder: &str) -> Card {
        Card { id: id.into(), content: content.into(), created_at: 1, updated_at: 1, folder: folder.into() }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn saving_again_keeps_the_file_name() {
        let root = temp_root();
        write_card(&root, &card("a", "# Plan\n\none", "")).unwrap();
        write_card(&root, &card("a", "# Plan\n\ntwo", "")).unwrap();
        assert_eq!(names(&root), ["Plan.md"]);

        // Another note with the same title gets a numbered name and keeps it
        write_card(&root, &card("b", "# Plan", "")).unwrap();
        write_card(&root, &card("b", "# Plan\n\nmore", "")).unwrap();
        assert_eq!(names(&root), ["Plan (2).md", "Plan.md"]);
    }

    #[test]
    fn a_new_title_renames_the_file() {
        let root = temp_root();
        write_card(&root, &card("a", "# Old", "")).unwrap();
        write_card(&root, &card("a", "# New", "")).unwrap();
        assert_eq!(names(&root), ["New.md"]);
    }

    #[test]
    fn notes_are_read_from_folders_and_move_between_them() {
        let root = temp_root();
        write_card(&root, &card("a", "# Loose", "")).unwrap();
        write_card(&root, &card("b", "# Filed", "Work")).unwrap();

        let mut cards = scan(&root).unwrap();
        cards.sort_by(|x, y| x.id.cmp(&y.id));
        assert_eq!(cards[0].folder, "");
        assert_eq!(cards[1].folder, "Work");

        write_card(&root, &card("a", "# Loose", "Work")).unwrap();
        assert_eq!(names(&root), ["Work"]);
        assert_eq!(names(&root.join("Work")), ["Filed.md", "Loose.md"]);

        write_card(&root, &card("b", "# Filed", "")).unwrap();
        assert_eq!(names(&root), ["Filed.md", "Work"]);
    }

    #[test]
    fn folders_are_created_renamed_and_removed_without_losing_notes() {
        let root = temp_root();
        assert_eq!(make_folder(&root, "  Work/Home ").unwrap(), "Work-Home");
        assert!(make_folder(&root, "  ").is_err());
        assert!(make_folder(&root, "..").is_err());
        assert!(make_folder(&root, ".hidden").is_err());

        make_folder(&root, "Ideas").unwrap();
        assert_eq!(folders_in(&root).unwrap(), ["Ideas", "Work-Home"]);
        assert!(move_folder(&root, "Ideas", "Work-Home").is_err());
        assert_eq!(move_folder(&root, "Ideas", "Later").unwrap(), "Later");

        write_card(&root, &card("a", "# Plan", "")).unwrap();
        write_card(&root, &card("b", "# Plan", "Later")).unwrap();
        remove_folder(&root, "Later").unwrap();
        assert_eq!(names(&root), ["Plan (2).md", "Plan.md", "Work-Home"]);
        assert_eq!(scan(&root).unwrap().len(), 2);
        assert!(remove_folder(&root, "Later").is_err());
        assert!(remove_folder(&root, "").is_err());
    }

    #[test]
    fn long_titles_with_multibyte_characters_do_not_panic() {
        let title = "ä".repeat(120);
        assert_eq!(sanitize_filename(&title).chars().count(), 100);
        assert!(extract_title_from_content(&title).ends_with("..."));
    }
}
