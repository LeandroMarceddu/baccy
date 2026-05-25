// Who-Has / I-Have search commands

use crate::state::AppState;
use baccy_core::DeviceId;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IHaveResult {
    pub device_id: DeviceId,
    pub object_type: String,
    pub object_instance: u32,
    pub object_name: String,
}

/// Send a Who-Has by name and collect all I-Have responses.
#[tauri::command]
pub async fn who_has_by_name(
    object_name: String,
    timeout_ms: u64,
    state: State<'_, AppState>,
) -> Result<Vec<IHaveResult>, String> {
    let service = {
        let guard = state.service.lock().unwrap();
        guard
            .as_ref()
            .ok_or_else(|| "BACnet service not initialized".to_string())
            .map(|s| s.clone())?
    };

    let results = tokio::task::spawn_blocking(move || {
        service
            .who_has_by_name(&object_name)
            .map_err(|e| format!("Failed to send Who-Has: {}", e.user_message()))?;

        let timeout = Duration::from_millis(timeout_ms);
        let start = std::time::Instant::now();
        let mut results = Vec::new();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                break;
            }
            let remaining = timeout - elapsed;

            match service.receive_ihave(remaining) {
                Ok(info) => {
                    results.push(IHaveResult {
                        device_id: info.device_id,
                        object_type: info.object_id.object_type.name().to_string(),
                        object_instance: info.object_id.instance,
                        object_name: info.object_name,
                    });
                }
                Err(baccy_protocol::ProtocolError::Timeout) => {
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error receiving I-Have: {}", e.user_message());
                    continue;
                }
            }
        }

        Ok::<Vec<IHaveResult>, String>(results)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    Ok(results)
}

/// Send a Who-Has by object identifier and collect all I-Have responses.
#[tauri::command]
pub async fn who_has_by_object(
    object_type: String,
    instance: u32,
    timeout_ms: u64,
    state: State<'_, AppState>,
) -> Result<Vec<IHaveResult>, String> {
    let service = {
        let guard = state.service.lock().unwrap();
        guard
            .as_ref()
            .ok_or_else(|| "BACnet service not initialized".to_string())
            .map(|s| s.clone())?
    };

    let baccy_core_obj_type = parse_object_type(&object_type)
        .ok_or_else(|| format!("Unknown object type: {}", object_type))?;

    let results = tokio::task::spawn_blocking(move || {
        service
            .who_has_by_object(baccy_core_obj_type, instance)
            .map_err(|e| format!("Failed to send Who-Has: {}", e.user_message()))?;

        let timeout = Duration::from_millis(timeout_ms);
        let start = std::time::Instant::now();
        let mut results = Vec::new();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                break;
            }
            let remaining = timeout - elapsed;

            match service.receive_ihave(remaining) {
                Ok(info) => {
                    results.push(IHaveResult {
                        device_id: info.device_id,
                        object_type: info.object_id.object_type.name().to_string(),
                        object_instance: info.object_id.instance,
                        object_name: info.object_name,
                    });
                }
                Err(baccy_protocol::ProtocolError::Timeout) => {
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error receiving I-Have: {}", e.user_message());
                    continue;
                }
            }
        }

        Ok::<Vec<IHaveResult>, String>(results)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    Ok(results)
}

fn parse_object_type(s: &str) -> Option<baccy_core::ObjectType> {
    baccy_core::ObjectType::from_display_name(s)
        .or_else(|| baccy_core::ObjectType::from_debug_name(s))
}
