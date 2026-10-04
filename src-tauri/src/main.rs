//! HexStickyNote - Next-Gen AI Workspace
//!
//! Main entry point for the Tauri v2 application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hex_sticky_note::ai_manager::AiManager;
use hex_sticky_note::calendar;
use hex_sticky_note::commands::*;
use hex_sticky_note::local_inference;
use hex_sticky_note::settings_manager::SettingsManager;
use std::sync::Arc;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting HexStickyNote...");

    // WebKitGTK's DMABUF renderer shows blank or broken transparent windows on
    // many Linux setups (notably NVIDIA + Wayland). Must be set before GTK starts.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    // Initialize llama backend for local models (non-fatal if it fails)
    if local_inference::init_backend() {
        log::info!("Llama backend initialized");
    } else {
        log::warn!("Llama backend not available - local AI features disabled");
    }

    // Initialize settings manager
    let settings = Arc::new(SettingsManager::new().expect("Failed to initialize settings"));
    log::info!("Settings manager initialized");

    let calendar_settings = settings.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AiManager::new(settings.clone()))
        .manage(settings)
        .setup(move |app| {
            // A calendar failure must not take the notes down with it
            match calendar::init(app, calendar_settings) {
                Ok(()) => log::info!("Calendar initialized"),
                Err(e) => log::error!("Calendar unavailable: {}", e),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Model Selection
            list_local_models,
            get_active_model,
            set_active_model,
            // AI Streaming
            invoke_ai_stream,
            // Card Storage
            create_card,
            get_cards,
            save_card,
            delete_card,
            reload_cards,
            // Settings
            get_all_settings,
            set_gpu_type,
            // Local Models
            download_model,
            cancel_model_download,
            delete_local_model,
            // Window State
            load_window_state,
            save_main_window_position,
            save_orb_window_position,
            // Application Control
            exit_app,
            // Claude Desktop MCP
            check_claude_mcp,
            setup_claude_mcp,
            remove_claude_mcp,
            // File System
            open_cards_directory,
            // Calendar (HexCalendar)
            calendar::commands::calendar::get_calendar_data,
            calendar::commands::calendar::create_event,
            calendar::commands::calendar::update_event,
            calendar::commands::calendar::delete_event,
            calendar::commands::calendar::get_day_notes,
            calendar::commands::calendar::save_day_notes,
            calendar::commands::ai::generate_day_briefing,
            calendar::commands::ai::generate_week_summary,
            calendar::commands::ai::stream_ai_response,
            calendar::commands::ai::get_cached_digest,
            calendar::commands::ai::invalidate_day_digest,
            calendar::commands::notifications::schedule_reminder,
            calendar::commands::notifications::dismiss_reminder,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running HexStickyNote");
}
