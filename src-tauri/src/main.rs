// Tauri main entry point for Baccy BACnet Browser
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::Manager;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting Baccy BACnet Browser");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize application state
            let state = AppState::new();
            app.manage(state);
            
            tracing::info!("Tauri application initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::network::get_network_interfaces,
            commands::network::initialize_service,
            commands::network::shutdown_service,
            commands::devices::discover_devices,
            commands::devices::get_devices,
            commands::objects::load_objects,
            commands::objects::get_objects,
            commands::objects::get_objects_by_type,
            commands::properties::load_properties,
            commands::properties::get_properties,
            commands::properties::update_property,
            commands::properties::refresh_properties,
            commands::trending::add_to_trending,
            commands::trending::remove_from_trending,
            commands::trending::get_trending_data,
            commands::trending::clear_trending,
            commands::trending::toggle_trending_visibility,
            commands::trending::poll_trending,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
