// Device discovery and management commands

use crate::state::AppState;
use baccy_core::Device;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub instance: u32,
    pub name: String,
    pub vendor_id: u16,
    pub vendor_name: String,
}

impl From<&Device> for DeviceInfo {
    fn from(device: &Device) -> Self {
        Self {
            instance: device.instance,
            name: device.name.clone(),
            vendor_id: device.vendor_id,
            vendor_name: device.vendor_name.clone(),
        }
    }
}

/// Discover BACnet devices within an instance range
#[tauri::command]
pub async fn discover_devices_range(
    low: u32,
    high: u32,
    state: State<'_, AppState>,
) -> Result<Vec<DeviceInfo>, String> {
    tracing::info!(low, high, "Starting device range discovery");

    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    let devices = tokio::task::spawn_blocking(move || {
        let mut manager = baccy_app::DeviceManager::new(service);
        manager.discover_devices_range(low, high)?;
        Ok::<Vec<baccy_core::Device>, baccy_app::AppError>(
            manager.list_devices().into_iter().cloned().collect()
        )
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Range discovery failed: {}", e))?;

    {
        let mut device_manager = state.device_manager.lock().unwrap();
        if let Some(manager) = device_manager.as_mut() {
            for device in &devices {
                manager.update_device(device.clone());
            }
        }
    }

    let device_infos: Vec<DeviceInfo> = devices.iter().map(DeviceInfo::from).collect();

    tracing::info!("Discovered {} devices in range [{}-{}]", device_infos.len(), low, high);
    Ok(device_infos)
}

/// Discover BACnet devices on the network
#[tauri::command]
pub async fn discover_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, String> {
    tracing::info!("Starting device discovery");
    
    // Clone the service Arc for use in the blocking task
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };
    
    // Run discovery in blocking task
    let devices = tokio::task::spawn_blocking(move || {
        let mut manager = baccy_app::DeviceManager::new(service);
        manager.discover_devices()?;
        Ok::<Vec<baccy_core::Device>, baccy_app::AppError>(
            manager.list_devices().into_iter().cloned().collect()
        )
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Discovery failed: {}", e))?;
    
    // Store devices in state
    {
        let mut device_manager = state.device_manager.lock().unwrap();
        if let Some(manager) = device_manager.as_mut() {
            for device in &devices {
                manager.update_device(device.clone());
            }
        }
    }
    
    let device_infos: Vec<DeviceInfo> = devices
        .iter()
        .map(DeviceInfo::from)
        .collect();
    
    tracing::info!("Discovered {} devices", device_infos.len());
    Ok(device_infos)
}

/// Get list of discovered devices
#[tauri::command]
pub fn get_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, String> {
    let device_manager = state.device_manager.lock().unwrap();
    let manager = device_manager
        .as_ref()
        .ok_or("BACnet service not initialized")?;
    
    let devices: Vec<DeviceInfo> = manager
        .list_devices()
        .iter()
        .map(|d| DeviceInfo::from(*d))
        .collect();
    
    Ok(devices)
}
