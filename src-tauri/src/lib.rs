//! HexStickyNote - Next-Gen AI Workspace Backend
//!
//! This library provides the Rust backend for the HexStickyNote application,
//! including note storage, the calendar, local AI inference and Claude Desktop MCP setup.

pub mod ai_manager;
pub mod calendar;
pub mod card_manager;
pub mod claude_mcp;
pub mod commands;
pub mod local_inference;
pub mod local_model;
pub mod ollama;
pub mod settings_manager;
pub mod window_state;

pub use ai_manager::AiManager;
pub use settings_manager::SettingsManager;
