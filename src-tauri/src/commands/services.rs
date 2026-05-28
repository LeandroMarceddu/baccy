use crate::state::AppState;
use baccy_protocol::{AtomicReadFileResult, TrendLogRecord};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicReadFileInfo {
    pub end_of_file: bool,
    pub file_start_position: i32,
    pub file_data_hex: String,
}

/// Read a file object via BACnet AtomicReadFile (confirmed service).
///
/// The raw file data is returned as a hex string because it
/// may not be valid UTF-8.
#[tauri::command]
pub fn atomic_read_file(
    device_id: u32,
    file_instance: u32,
    start_position: i32,
    octet_count: u32,
    state: State<'_, AppState>,
) -> Result<AtomicReadFileInfo, String> {
    let service = state
        .service
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("BACnet service not initialized")?
        .clone();

    let object_id = baccy_core::ObjectId {
        object_type: baccy_core::ObjectType::File,
        instance: file_instance,
    };

    tracing::info!(
        device_id,
        file_instance,
        start_position,
        octet_count,
        "AtomicReadFile"
    );

    let result: AtomicReadFileResult = service
        .atomic_read_file(device_id, object_id, start_position, octet_count)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(AtomicReadFileInfo {
        end_of_file: result.end_of_file,
        file_start_position: result.file_start_position,
        file_data_hex: hex::encode(result.file_data),
    })
}

/// Write to a file object via BACnet AtomicWriteFile (confirmed service).
///
/// The file data is provided as a hex string and decoded before sending.
/// Returns the new file start position from the device.
#[tauri::command]
pub fn atomic_write_file(
    device_id: u32,
    file_instance: u32,
    start_position: i32,
    data_hex: String,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    let service = state
        .service
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("BACnet service not initialized")?
        .clone();

    let object_id = baccy_core::ObjectId {
        object_type: baccy_core::ObjectType::File,
        instance: file_instance,
    };

    let data = hex::decode(&data_hex).map_err(|e| format!("Invalid hex data: {}", e))?;

    tracing::info!(
        device_id,
        file_instance,
        start_position,
        data_len = data.len(),
        "AtomicWriteFile"
    );

    service
        .atomic_write_file(device_id, object_id, start_position, data)
        .map_err(|e| format!("Failed to write file: {}", e))
}

/// Send an UnconfirmedPrivateTransfer message as a global broadcast.
#[tauri::command]
pub fn send_unconfirmed_private_transfer(
    vendor_id: u16,
    service_number: u32,
    service_data_hex: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let service = state
        .service
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("BACnet service not initialized")?
        .clone();

    let data = service_data_hex
        .map(|h| hex::decode(&h).map_err(|e| format!("Invalid hex data: {}", e)))
        .transpose()?;

    tracing::info!(
        vendor_id,
        service_number,
        data_len = data.as_ref().map(|d| d.len()).unwrap_or(0),
        "UnconfirmedPrivateTransfer"
    );

    service
        .send_unconfirmed_private_transfer(vendor_id, service_number, data)
        .map_err(|e| format!("Failed to send private transfer: {}", e))
}

/// Read the LogBuffer from a TrendLog object and return parsed records.
#[tauri::command]
pub fn read_trend_log_buffer(
    device_id: u32,
    object_instance: u32,
    state: State<'_, AppState>,
) -> Result<Vec<TrendLogRecord>, String> {
    let service = state
        .service
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("BACnet service not initialized")?
        .clone();

    let object_id = baccy_core::ObjectId {
        object_type: baccy_core::ObjectType::TrendLog,
        instance: object_instance,
    };

    tracing::info!(device_id, object_instance, "Reading TrendLog buffer");

    service
        .read_trend_log_buffer(device_id, object_id)
        .map_err(|e| format!("Failed to read TrendLog buffer: {}", e))
}
