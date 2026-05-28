use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FdtEntryInfo {
    pub address: String,
    pub time_to_live: u32,
    pub remaining_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BbmdStatusInfo {
    pub enabled: bool,
    pub registered_to: Option<String>,
    pub last_registration_ms: Option<u64>,
    pub ttl: Option<u32>,
    pub fdt_entries: Vec<FdtEntryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BbmdRegistrationResult {
    pub success: bool,
    pub message: String,
}

/// Get BBMD status including FDT entries and registration state
#[tauri::command]
pub fn get_bbmd_status(state: State<'_, AppState>) -> Result<BbmdStatusInfo, String> {
    let bbmd_state = state.bbmd_state.lock().unwrap();

    let fdt_entries = bbmd_state
        .fdt_entries
        .iter()
        .map(|e| {
            let elapsed = e.registration_time.elapsed().as_secs() as u32;
            let remaining = e.time_to_live.saturating_sub(elapsed);
            FdtEntryInfo {
                address: e.address.to_string(),
                time_to_live: e.time_to_live,
                remaining_seconds: remaining,
            }
        })
        .collect();

    let last_registration_ms = bbmd_state.last_registration.map(|t| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let elapsed_ms = t.elapsed().as_millis() as u64;
        now.saturating_sub(elapsed_ms)
    });

    Ok(BbmdStatusInfo {
        enabled: bbmd_state.enabled,
        registered_to: bbmd_state.bbmd_address.as_ref().map(|a| a.to_string()),
        last_registration_ms,
        ttl: bbmd_state.ttl,
        fdt_entries,
    })
}

/// Enable BBMD mode (updates application state)
#[tauri::command]
pub fn start_bbmd(state: State<'_, AppState>) -> Result<(), String> {
    let mut bbmd_state = state.bbmd_state.lock().unwrap();
    bbmd_state.enabled = true;
    tracing::info!("BBMD enabled");
    Ok(())
}

/// Disable BBMD mode
#[tauri::command]
pub fn stop_bbmd(state: State<'_, AppState>) -> Result<(), String> {
    let mut bbmd_state = state.bbmd_state.lock().unwrap();
    bbmd_state.enabled = false;
    bbmd_state.bbmd_address = None;
    bbmd_state.last_registration = None;
    bbmd_state.ttl = None;
    bbmd_state.fdt_entries.clear();
    tracing::info!("BBMD disabled");
    Ok(())
}

/// Register as a foreign device with a remote BBMD
#[tauri::command]
pub fn register_as_foreign_device(
    bbmd_ip: String,
    ttl: u32,
    state: State<'_, AppState>,
) -> Result<BbmdRegistrationResult, String> {
    let bbmd_addr: std::net::SocketAddr = format!("{}:47808", bbmd_ip)
        .parse()
        .map_err(|e| format!("Invalid BBMD IP address '{}': {}", bbmd_ip, e))?;

    let mut bbmd_state = state.bbmd_state.lock().unwrap();
    bbmd_state.bbmd_address = Some(bbmd_addr);
    bbmd_state.ttl = Some(ttl);
    bbmd_state.last_registration = Some(std::time::Instant::now());
    bbmd_state.enabled = true;

    tracing::info!(address = %bbmd_addr, ttl, "Foreign device registration recorded");
    Ok(BbmdRegistrationResult {
        success: true,
        message: format!("Registered with BBMD at {} with TTL {}s", bbmd_addr, ttl),
    })
}

/// Get the FDT (Foreign Device Table) entries
#[tauri::command]
pub fn get_fdt(state: State<'_, AppState>) -> Result<Vec<FdtEntryInfo>, String> {
    let bbmd_state = state.bbmd_state.lock().unwrap();
    let entries = bbmd_state
        .fdt_entries
        .iter()
        .map(|e| {
            let elapsed = e.registration_time.elapsed().as_secs() as u32;
            let remaining = e.time_to_live.saturating_sub(elapsed);
            FdtEntryInfo {
                address: e.address.to_string(),
                time_to_live: e.time_to_live,
                remaining_seconds: remaining,
            }
        })
        .collect();
    Ok(entries)
}

/// Add a foreign device entry to the FDT
#[tauri::command]
pub fn add_fd_entry(
    address: String,
    time_to_live: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let addr: std::net::SocketAddr = address
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;
    let mut bbmd_state = state.bbmd_state.lock().unwrap();
    let entry = baccy_transport::bbmd::ForeignDeviceEntry {
        address: addr,
        time_to_live,
        registration_time: std::time::Instant::now(),
    };
    bbmd_state.fdt_entries.push(entry);
    tracing::info!(address = %addr, ttl = time_to_live, "Foreign device entry added");
    Ok(())
}

/// Remove a foreign device entry from the FDT by address
#[tauri::command]
pub fn remove_fd_entry(address: String, state: State<'_, AppState>) -> Result<(), String> {
    let addr: std::net::SocketAddr = address
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;
    let mut bbmd_state = state.bbmd_state.lock().unwrap();
    let len_before = bbmd_state.fdt_entries.len();
    bbmd_state.fdt_entries.retain(|e| e.address != addr);
    if bbmd_state.fdt_entries.len() == len_before {
        return Err(format!("Entry with address {} not found", addr));
    }
    tracing::info!(address = %addr, "Foreign device entry removed");
    Ok(())
}

/// Clear all foreign device entries
#[tauri::command]
pub fn clear_fdt(state: State<'_, AppState>) -> Result<(), String> {
    let mut bbmd_state = state.bbmd_state.lock().unwrap();
    bbmd_state.fdt_entries.clear();
    tracing::info!("Foreign device table cleared");
    Ok(())
}
