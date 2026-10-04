//! HexStickyNote - Next-Gen AI Workspace Backend
//!
//! This library provides the Rust backend for the HexStickyNote application,
//! including note storage, the calendar, the Ollama-based assistant and Claude Desktop MCP setup.

pub mod assistant;
pub mod calendar;
pub mod card_manager;
pub mod claude_mcp;
pub mod commands;
pub mod hextime;
pub mod http;
pub mod ollama;
pub mod settings_manager;
pub mod window_state;

pub use settings_manager::SettingsManager;
