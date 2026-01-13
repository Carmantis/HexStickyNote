//! Window State Management
//!
//! Saves and loads window positions to maintain state across app restarts.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowState {
    pub main_window: Option<WindowPosition>,
    pub orb_window: Option<WindowPosition>,
}

impl WindowState {
    /// Get the path to the window state file
    fn get_state_file_path() -> Result<PathBuf, String> {
        let proj_dirs = ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
            .ok_or_else(|| "Failed to get config directory".to_string())?;

        let config_dir = proj_dirs.config_dir();

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        Ok(config_dir.join("window_state.json"))
    }

    /// Load window state from disk
    pub fn load() -> Result<Self, String> {
        let path = Self::get_state_file_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read window state file: {}", e))?;

        let state: WindowState = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse window state: {}", e))?;

        Ok(state)
    }

    /// Save window state to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_state_file_path()?;

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize window state: {}", e))?;

        fs::write(&path, contents)
            .map_err(|e| format!("Failed to write window state file: {}", e))?;

        Ok(())
    }

    /// Update main window position
    pub fn set_main_position(&mut self, x: i32, y: i32) {
        self.main_window = Some(WindowPosition { x, y });
    }

    /// Update orb window position
    pub fn set_orb_position(&mut self, x: i32, y: i32) {
        self.orb_window = Some(WindowPosition { x, y });
    }
}
