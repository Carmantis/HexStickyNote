//! HexStickyNote - Next-Gen AI Workspace
//!
//! Main entry point for the Tauri v2 application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hex_sticky_note::ai_manager::AiManager;
use hex_sticky_note::commands::*;
use tauri::Manager;

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
            // Window State
            load_window_state,
            save_main_window_position,
            save_orb_window_position,
            // Application Control
            exit_app,
        ])
        .setup(|app| {
            // Route orb window to /orb page
            if let Some(orb_window) = app.get_webview_window("orb") {
                let _ = orb_window.eval("window.location.href = '/orb'");
                log::info!("Orb window routed to /orb");
            } else {
                log::warn!("Orb window not found during setup");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running HexStickyNote");
}
