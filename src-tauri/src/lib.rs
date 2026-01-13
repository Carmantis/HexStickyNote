//! HexStickyNote - Next-Gen AI Workspace Backend
//!
//! This library provides the Rust backend for the HexStickyNote application,
//! including secure API key storage and AI provider integration.

pub mod ai_manager;
pub mod ai_tools;
pub mod card_manager;
pub mod commands;
pub mod keyring_store;
pub mod window_state;

pub use ai_manager::AiManager;
pub use keyring_store::{AiProvider, KeyringStore};
