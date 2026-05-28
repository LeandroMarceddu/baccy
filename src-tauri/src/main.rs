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
            commands::compare::compare_objects,
            commands::network::get_network_interfaces,
            commands::network::get_serial_ports,
            commands::network::initialize_service,
            commands::network::initialize_service_bbmd,
            commands::network::connect_bacnet_mstp,
            commands::network::shutdown_service,
            commands::devices::discover_devices,
            commands::devices::discover_devices_range,
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
            commands::trending::export_trending_csv,
            commands::trending::export_trending_parquet,
            commands::search::who_has_by_name,
            commands::search::who_has_by_object,
            commands::cov::subscribe_cov,
            commands::cov::subscribe_cov_property,
            commands::cov::unsubscribe_cov,
            commands::cov::poll_cov_notifications,
            commands::packet::get_packet_log,
            commands::packet::clear_packet_log,
            commands::packet::set_packet_logging,
            commands::device::get_device_info,
            commands::device::reinitialize_device,
            commands::device::device_communication_control,
            commands::health::get_device_health,
            commands::network::get_network_stats,
            commands::network::get_throttle_status,
            commands::export::export_device_config,
            commands::export::import_device_config,
            commands::write_prefs::is_write_protected,
            commands::write_prefs::set_write_protection,
            commands::write_prefs::get_all_write_protections,
            // BBMD commands
            commands::bbmd::get_bbmd_status,
            commands::bbmd::start_bbmd,
            commands::bbmd::stop_bbmd,
            commands::bbmd::register_as_foreign_device,
            commands::bbmd::get_fdt,
            commands::bbmd::add_fd_entry,
            commands::bbmd::remove_fd_entry,
            commands::bbmd::clear_fdt,
            // Router commands
            commands::router::get_router_routes,
            commands::router::get_router_interfaces,
            commands::router::add_router_route,
            commands::router::remove_router_route,
            // Schedule/Calendar commands
            commands::schedule::read_schedule_data,
            commands::schedule::read_calendar_data,
            // File Access & Private Transfer commands
            commands::services::atomic_read_file,
            commands::services::atomic_write_file,
            commands::services::send_unconfirmed_private_transfer,
            commands::services::read_trend_log_buffer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
