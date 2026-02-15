//! Claude Desktop MCP Configuration
//!
//! Manages the Claude Desktop `claude_desktop_config.json` to register
//! HexStickyNote's MCP server for AI tool integration.

use serde_json::{json, Value};
use std::path::PathBuf;

/// Get the Claude Desktop config file path
fn get_claude_config_path() -> Result<PathBuf, String> {
    let app_data = std::env::var("APPDATA")
        .map_err(|_| "APPDATA environment variable not set".to_string())?;
    Ok(PathBuf::from(app_data).join("Claude").join("claude_desktop_config.json"))
}

/// Get the path to the bundled MCP server
fn get_mcp_server_path(app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let resource_path = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?
        .join("resources")
        .join("hexstickynote-mcp.mjs");

    // Convert to normal Windows path (remove UNC prefix if present)
    let path_str = resource_path.to_string_lossy().to_string();
    let normalized = if path_str.starts_with(r"\\?\") {
        path_str[4..].to_string()
    } else {
        path_str
    };

    Ok(normalized)
}

/// Check status of Claude Desktop MCP integration
#[derive(serde::Serialize)]
pub struct ClaudeMcpStatus {
    /// Whether Claude Desktop config directory exists
    pub claude_installed: bool,
    /// Whether HexStickyNote MCP is configured
    pub mcp_configured: bool,
    /// Path to the MCP server bundle
    pub mcp_server_path: String,
}

/// Check if Claude Desktop is installed and MCP is configured
pub fn check_status(app: &tauri::AppHandle) -> Result<ClaudeMcpStatus, String> {
    let config_path = get_claude_config_path()?;
    let claude_installed = config_path.parent().map_or(false, |p| p.exists());

    let mcp_server_path = get_mcp_server_path(app).unwrap_or_default();

    let mcp_configured = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let config: Value = serde_json::from_str(&content)
            .unwrap_or(json!({}));
        config.get("mcpServers")
            .and_then(|s| s.get("hexstickynote"))
            .is_some()
    } else {
        false
    };

    Ok(ClaudeMcpStatus {
        claude_installed,
        mcp_configured,
        mcp_server_path,
    })
}

/// Add HexStickyNote MCP to Claude Desktop config
pub fn setup(app: &tauri::AppHandle) -> Result<(), String> {
    let config_path = get_claude_config_path()?;
    let mcp_server_path = get_mcp_server_path(app)?;

    // Ensure Claude config directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Claude config dir: {}", e))?;
    }

    // Read existing config or start fresh
    let mut config: Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&content)
            .unwrap_or(json!({}))
    } else {
        json!({})
    };

    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = json!({});
    }

    // Add/update hexstickynote entry
    config["mcpServers"]["hexstickynote"] = json!({
        "command": "node",
        "args": [mcp_server_path]
    });

    // Write back
    let formatted = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, formatted)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    log::info!("Claude Desktop MCP configured at {:?}", config_path);
    Ok(())
}

/// Remove HexStickyNote MCP from Claude Desktop config
pub fn remove() -> Result<(), String> {
    let config_path = get_claude_config_path()?;

    if !config_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let mut config: Value = serde_json::from_str(&content)
        .unwrap_or(json!({}));

    // Remove hexstickynote entry
    if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
        servers.remove("hexstickynote");
    }

    let formatted = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, formatted)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    log::info!("Claude Desktop MCP removed");
    Ok(())
}
