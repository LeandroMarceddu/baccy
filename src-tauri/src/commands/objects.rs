// Object loading and management commands

use crate::state::AppState;
use baccy_core::BacnetObject;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub object_type: String,
    pub instance: u32,
    pub name: String,
}

impl From<&BacnetObject> for ObjectInfo {
    fn from(object: &BacnetObject) -> Self {
        Self {
            object_type: object.object_type.name().to_string(),
            instance: object.instance,
            name: object.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectGroup {
    pub object_type: String,
    pub objects: Vec<ObjectInfo>,
}

/// Load objects from a device
#[tauri::command]
pub async fn load_objects(
    device_id: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ObjectInfo>, String> {
    tracing::info!(device_id, "Loading objects for device");
    
    // Clone the service Arc for use in the blocking task
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };
    
    // Run load_objects in blocking task
    let objects = tokio::task::spawn_blocking(move || {
        let mut manager = baccy_app::ObjectManager::new(service);
        manager.load_objects(device_id)?;
        Ok::<Vec<baccy_core::BacnetObject>, baccy_app::AppError>(
            manager.list_objects().into_iter().cloned().collect()
        )
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Failed to load objects: {}", e))?;
    
    // Sync state manager
    {
        let service = {
            let service_lock = state.service.lock().unwrap();
            service_lock.as_ref().map(|s| s.clone())
        };
        if let Some(_svc) = service {
            let object_manager = state.object_manager.clone();
            tokio::task::spawn_blocking(move || {
                if let Ok(mut manager_guard) = object_manager.lock() {
                    if let Some(manager) = manager_guard.as_mut() {
                        let _ = manager.load_objects(device_id);
                    }
                }
            });
        }
    }
    
    let object_infos: Vec<ObjectInfo> = objects
        .iter()
        .map(ObjectInfo::from)
        .collect();
    
    tracing::info!(device_id, object_count = object_infos.len(), "Objects loaded successfully");
    Ok(object_infos)
}

/// Get currently loaded objects
#[tauri::command]
pub fn get_objects(state: State<'_, AppState>) -> Result<Vec<ObjectInfo>, String> {
    let object_manager = state.object_manager.lock().unwrap();
    let manager = object_manager
        .as_ref()
        .ok_or("No objects loaded")?;
    
    let objects: Vec<ObjectInfo> = manager
        .list_objects()
        .iter()
        .map(|o| ObjectInfo::from(*o))
        .collect();
    
    Ok(objects)
}

/// Get objects grouped by type
#[tauri::command]
pub fn get_objects_by_type(state: State<'_, AppState>) -> Result<Vec<ObjectGroup>, String> {
    let object_manager = state.object_manager.lock().unwrap();
    let manager = object_manager
        .as_ref()
        .ok_or("No objects loaded")?;
    
    let grouped = manager.group_by_type();
    let mut result: Vec<ObjectGroup> = grouped
        .into_iter()
        .map(|(obj_type, objects)| ObjectGroup {
            object_type: obj_type.name().to_string(),
            objects: objects.iter().map(|o| ObjectInfo::from(*o)).collect(),
        })
        .collect();
    
    // Sort by object type name for consistent ordering
    result.sort_by(|a, b| a.object_type.cmp(&b.object_type));
    
    Ok(result)
}
