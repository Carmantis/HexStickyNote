//! HexStickyNote - Next-Gen AI Workspace
//!
//! Main entry point for the Tauri v2 application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hex_sticky_note::ai_manager::AiManager;
use hex_sticky_note::commands::*;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting HexStickyNote...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AiManager::new())
        .invoke_handler(tauri::generate_handler![
            // API Key Management
            save_api_key,
            delete_api_key,
            get_providers,
            set_active_provider,
            get_active_provider,
            // AI Streaming
            invoke_ai_stream,
            // Card Storage
            create_card,
            get_cards,
            save_card,
            delete_card,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running HexStickyNote");
}
