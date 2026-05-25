// Network interface selection and initialization commands

use crate::state::AppState;
use baccy_transport::network_stats::NetworkStats;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialPortInfo {
    pub port_name: String,
    pub port_type: String,
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

/// Get available serial ports for MS/TP communication
#[tauri::command]
pub fn get_serial_ports() -> Result<Vec<SerialPortInfo>, String> {
    tracing::info!("Getting available serial ports");
    
    match serialport::available_ports() {
        Ok(ports) => {
            let port_list: Vec<SerialPortInfo> = ports
                .into_iter()
                .map(|port| {
                    let port_type = match port.port_type {
                        serialport::SerialPortType::UsbPort(_) => "USB".to_string(),
                        serialport::SerialPortType::PciPort => "PCI".to_string(),
                        serialport::SerialPortType::BluetoothPort => "Bluetooth".to_string(),
                        serialport::SerialPortType::Unknown => "Unknown".to_string(),
                    };
                    
                    SerialPortInfo {
                        port_name: port.port_name,
                        port_type,
                    }
                })
                .collect();
            
            tracing::info!("Found {} serial ports", port_list.len());
            Ok(port_list)
        }
        Err(e) => {
            tracing::error!("Failed to enumerate serial ports: {}", e);
            Err(format!("Failed to enumerate serial ports: {}", e))
        }
    }
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

/// Connect to BACnet network using MS/TP transport
#[tauri::command]
pub fn connect_bacnet_mstp(
    port_name: String,
    baud_rate: u32,
    local_mac: u8,
    timeout_ms: u64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(
        port_name,
        baud_rate,
        local_mac,
        timeout_ms,
        "Connecting to BACnet MS/TP network"
    );
    
    // Validate MAC address (0-127 for master nodes)
    if local_mac > 127 {
        return Err(format!(
            "Invalid MAC address: {}. Master nodes must use MAC addresses 0-127.",
            local_mac
        ));
    }
    
    // Validate baud rate
    let valid_baud_rates = [9600, 19200, 38400, 76800, 115200];
    if !valid_baud_rates.contains(&baud_rate) {
        return Err(format!(
            "Invalid baud rate: {}. Supported rates: 9600, 19200, 38400, 76800, 115200",
            baud_rate
        ));
    }
    
    state.initialize_mstp_service(port_name, baud_rate, local_mac, timeout_ms)?;
    
    tracing::info!("MS/TP service initialized successfully");
    Ok(())
}

/// Initialize BACnet service with BBMD support
#[tauri::command]
pub fn initialize_service_bbmd(
    ip: String,
    port: u16,
    timeout_ms: u64,
    bbmd_enabled: bool,
    bbmd_address: Option<String>,
    bbmd_port: Option<u16>,
    bbmd_ttl: Option<u32>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(
        ip,
        port,
        timeout_ms,
        bbmd_enabled,
        bbmd_addr = ?bbmd_address,
        bbmd_port,
        "Initializing BACnet service with BBMD"
    );

    let ipv4: Ipv4Addr = ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;

    let bbmd_addr = match (bbmd_address, bbmd_port) {
        (Some(addr), Some(bp)) => {
            let full = format!("{}:{}", addr, bp);
            Some(full.parse::<std::net::SocketAddr>()
                .map_err(|e| format!("Invalid BBMD address: {}", e))?)
        }
        (Some(addr), None) => {
            let full = format!("{}:47808", addr);
            Some(full.parse::<std::net::SocketAddr>()
                .map_err(|e| format!("Invalid BBMD address: {}", e))?)
        }
        (None, _) => None,
    };

    let ttl = bbmd_ttl.unwrap_or(120);

    state.initialize_bbmd_service(ipv4, port, timeout_ms, bbmd_enabled, bbmd_addr, ttl)?;

    tracing::info!("BBMD service initialized successfully");
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

/// Get current network statistics
#[tauri::command]
pub fn get_network_stats(state: State<'_, AppState>) -> Result<NetworkStats, String> {
    Ok(state.stats.snapshot())
}

/// Get per-device throttle concurrency counts
#[tauri::command]
pub fn get_throttle_status(state: State<'_, AppState>) -> Result<std::collections::HashMap<u32, usize>, String> {
    let service_guard = state.service.lock().unwrap();
    match service_guard.as_ref() {
        Some(service) => Ok(service.throttle().all_concurrency()),
        None => Ok(std::collections::HashMap::new()),
    }
}
