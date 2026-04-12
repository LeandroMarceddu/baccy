// Network interface selection and initialization commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
}

/// Get available network interfaces
#[tauri::command]
pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, String> {
    tracing::info!("Getting network interfaces");
    
    let mut interfaces = Vec::new();
    
    // Get all network interfaces
    match if_addrs::get_if_addrs() {
        Ok(if_addrs) => {
            for iface in if_addrs {
                // Only include IPv4 addresses that are not loopback
                if let if_addrs::IfAddr::V4(ref addr) = iface.addr {
                    if !addr.ip.is_loopback() {
                        interfaces.push(NetworkInterface {
                            name: iface.name.clone(),
                            ip: addr.ip.to_string(),
                        });
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get network interfaces: {}", e);
            return Err(format!("Failed to get network interfaces: {}", e));
        }
    }
    
    tracing::info!("Found {} network interfaces", interfaces.len());
    Ok(interfaces)
}

/// Initialize BACnet service with selected network interface
#[tauri::command]
pub fn initialize_service(
    ip: String,
    port: u16,
    timeout_ms: u64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(ip, port, timeout_ms, "Initializing BACnet service");
    
    let ipv4: Ipv4Addr = ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;
    
    state.initialize_service(ipv4, port, timeout_ms)?;
    
    tracing::info!("BACnet service initialized successfully");
    Ok(())
}

/// Shutdown the current service
#[tauri::command]
pub fn shutdown_service(state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Shutting down service");
    
    // Clear all managers
    *state.device_manager.lock().unwrap() = None;
    *state.object_manager.lock().unwrap() = None;
    *state.property_manager.lock().unwrap() = None;
    *state.trending_manager.lock().unwrap() = None;
    *state.service.lock().unwrap() = None;
    
    tracing::info!("Service shut down successfully");
    Ok(())
}
