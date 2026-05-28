// Device info command

use crate::state::AppState;
use baccy_core::{ObjectId, ObjectType, PropertyId};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceInfo {
    pub vendor_name: String,
    pub model_name: String,
    pub firmware_revision: String,
    pub app_software_version: String,
    pub protocol_version: u32,
    pub protocol_revision: u32,
    pub description: String,
    pub location: String,
    pub database_revision: u32,
    pub max_apdu_length: u32,
    pub apdu_timeout: u32,
    pub apdu_segment_timeout: u32,
}

fn try_read_string(
    service: &baccy_protocol::BacnetService,
    device_id: u32,
    dev_object: ObjectId,
    prop: PropertyId,
) -> String {
    match service.read_property(device_id, dev_object, prop) {
        Ok(baccy_core::PropertyValue::String(s)) => s,
        _ => String::new(),
    }
}

fn try_read_u32(
    service: &baccy_protocol::BacnetService,
    device_id: u32,
    dev_object: ObjectId,
    prop: PropertyId,
) -> u32 {
    match service.read_property(device_id, dev_object, prop) {
        Ok(baccy_core::PropertyValue::Unsigned(u)) => u as u32,
        Ok(baccy_core::PropertyValue::Integer(i)) => i as u32,
        Ok(baccy_core::PropertyValue::String(s)) => s.parse::<u32>().unwrap_or(0),
        _ => 0,
    }
}

/// Read rich device info from a BACnet device (its Device object).
#[tauri::command]
pub async fn get_device_info(
    device_id: u32,
    state: State<'_, AppState>,
) -> Result<DeviceInfo, String> {
    tracing::info!(device_id, "Reading device info");

    let mut info = DeviceInfo::default();

    // Fill from cached device if available
    {
        let device_manager = state.device_manager.lock().unwrap();
        if let Some(manager) = device_manager.as_ref() {
            if let Some(device) = manager.get_device(device_id) {
                info.vendor_name = device.vendor_name.clone();
                info.model_name = device.model_name.clone();
                info.description = device.description.clone();
            }
        }
    }

    // Try to read additional properties from the device
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    let dev_object = ObjectId {
        object_type: ObjectType::Device,
        instance: device_id,
    };

    // Read properties in a blocking task
    let mut result = tokio::task::spawn_blocking(move || {
        let mut info = DeviceInfo::default();

        info.vendor_name = try_read_string(&service, device_id, dev_object, PropertyId::VendorName);
        info.model_name = try_read_string(&service, device_id, dev_object, PropertyId::ModelName);
        info.firmware_revision =
            try_read_string(&service, device_id, dev_object, PropertyId::FirmwareRevision);
        info.app_software_version =
            try_read_string(&service, device_id, dev_object, PropertyId::AppSoftwareRevision);
        info.description =
            try_read_string(&service, device_id, dev_object, PropertyId::Description);
        info.location = try_read_string(&service, device_id, dev_object, PropertyId::Location);
        info.protocol_version =
            try_read_u32(&service, device_id, dev_object, PropertyId::ProtocolVersion);
        info.protocol_revision =
            try_read_u32(&service, device_id, dev_object, PropertyId::ProtocolRevision);
        info.database_revision =
            try_read_u32(&service, device_id, dev_object, PropertyId::DatabaseRevision);
        info.max_apdu_length =
            try_read_u32(&service, device_id, dev_object, PropertyId::MaxApduLengthAccepted);
        info.apdu_timeout =
            try_read_u32(&service, device_id, dev_object, PropertyId::ApduTimeout);
        info.apdu_segment_timeout =
            try_read_u32(&service, device_id, dev_object, PropertyId::ApduSegmentTimeout);

        info
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?;

    // Merge: cached values take priority for fields we already have
    if !info.vendor_name.is_empty() {
        result.vendor_name = info.vendor_name;
    }
    if !info.model_name.is_empty() {
        result.model_name = info.model_name;
    }
    if !info.description.is_empty() {
        result.description = info.description;
    }

    tracing::info!(
        device_id,
        vendor = %result.vendor_name,
        model = %result.model_name,
        "Device info read successfully"
    );

    Ok(result)
}

/// Reinitialize a BACnet device (coldstart/warmstart/backup/restore)
#[tauri::command]
pub async fn reinitialize_device(
    device_id: u32,
    reinit_state: u32,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(device_id, reinit_state, "Reinitializing device");

    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    tokio::task::spawn_blocking(move || {
        service
            .reinitialize_device(device_id, reinit_state, password.as_deref())
            .map_err(|e| format!("ReinitializeDevice failed: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Enable or disable communication with a BACnet device
#[tauri::command]
pub async fn device_communication_control(
    device_id: u32,
    enable: bool,
    time_duration: Option<u32>,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(device_id, enable, "Setting device communication control");

    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    tokio::task::spawn_blocking(move || {
        service
            .device_communication_control(device_id, time_duration, enable, password.as_deref())
            .map_err(|e| format!("DeviceCommunicationControl failed: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}
