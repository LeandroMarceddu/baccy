// COV (Change of Value) subscription commands

use crate::state::AppState;
use baccy_core::{ObjectId, ObjectType, PropertyId, PropertyValue};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovNotificationInfo {
    pub device_id: u32,
    pub subscriber_process_id: u32,
    pub object_type: String,
    pub object_instance: u32,
    pub time_remaining: Option<u32>,
    pub changed_values: Vec<ChangedValueInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedValueInfo {
    pub property_id: String,
    pub value: String,
    pub value_type: String,
}

fn parse_object_type(type_str: &str) -> Result<ObjectType, String> {
    match type_str {
        "Analog Input" | "AnalogInput" => Ok(ObjectType::AnalogInput),
        "Analog Output" | "AnalogOutput" => Ok(ObjectType::AnalogOutput),
        "Analog Value" | "AnalogValue" => Ok(ObjectType::AnalogValue),
        "Binary Input" | "BinaryInput" => Ok(ObjectType::BinaryInput),
        "Binary Output" | "BinaryOutput" => Ok(ObjectType::BinaryOutput),
        "Binary Value" | "BinaryValue" => Ok(ObjectType::BinaryValue),
        "Device" => Ok(ObjectType::Device),
        "Multi-State Input" | "MultiStateInput" => Ok(ObjectType::MultiStateInput),
        "Multi-State Output" | "MultiStateOutput" => Ok(ObjectType::MultiStateOutput),
        "Multi-State Value" | "MultiStateValue" => Ok(ObjectType::MultiStateValue),
        _ => Err(format!("Unknown object type: {}", type_str)),
    }
}

fn parse_property_id(id_str: &str) -> Result<PropertyId, String> {
    let normalized = id_str.replace(" ", "");
    match normalized.as_str() {
        "PresentValue" => Ok(PropertyId::PresentValue),
        "ObjectName" => Ok(PropertyId::ObjectName),
        "Description" => Ok(PropertyId::Description),
        "Units" => Ok(PropertyId::Units),
        "StatusFlags" => Ok(PropertyId::StatusFlags),
        "OutOfService" => Ok(PropertyId::OutOfService),
        "Reliability" => Ok(PropertyId::Reliability),
        "EventState" => Ok(PropertyId::EventState),
        "Priority" => Ok(PropertyId::Priority),
        _ => Err(format!("Unknown property ID: {}", id_str)),
    }
}

fn property_value_to_string(value: &PropertyValue) -> (String, String) {
    match value {
        PropertyValue::Real(v) => (v.to_string(), "Real".to_string()),
        PropertyValue::Integer(v) => (v.to_string(), "Integer".to_string()),
        PropertyValue::Unsigned(v) => (v.to_string(), "Unsigned".to_string()),
        PropertyValue::Boolean(v) => (v.to_string(), "Boolean".to_string()),
        PropertyValue::String(v) => (v.clone(), "String".to_string()),
        PropertyValue::Enumerated(v) => (v.to_string(), "Enumerated".to_string()),
        PropertyValue::BitString(v) => {
            let s: String = v.iter().map(|b| if *b { "1" } else { "0" }).collect();
            (s, "BitString".to_string())
        }
        PropertyValue::ObjectIdentifier { object_type, instance } => {
            (format!("{:?}({})", object_type, instance), "ObjectIdentifier".to_string())
        }
    }
}

/// Subscribe for COV on an object
#[tauri::command]
pub async fn subscribe_cov(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    lifetime_seconds: Option<u32>,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId {
        object_type: obj_type,
        instance: object_instance,
    };

    let cov_manager = state.cov_manager.clone();

    tokio::task::spawn_blocking(move || -> Result<u32, String> {
        let manager = cov_manager.lock().unwrap();
        let manager = manager.as_ref().ok_or("Service not initialized")?;

        let process_id = manager
            .subscribe(device_id, object_id, lifetime_seconds, Box::new(|_| {}))
            .map_err(|e| format!("Failed to subscribe COV: {}", e))?;

        tracing::info!(
            device_id,
            object_type,
            object_instance,
            process_id,
            "COV subscription created"
        );

        Ok(process_id)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Subscribe for COV on a specific property
#[tauri::command]
pub async fn subscribe_cov_property(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    property_id: String,
    lifetime_seconds: Option<u32>,
    cov_increment: Option<f32>,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId {
        object_type: obj_type,
        instance: object_instance,
    };
    let prop_id = parse_property_id(&property_id)?;

    let cov_manager = state.cov_manager.clone();

    tokio::task::spawn_blocking(move || -> Result<u32, String> {
        let manager = cov_manager.lock().unwrap();
        let manager = manager.as_ref().ok_or("Service not initialized")?;

        let process_id = manager
            .subscribe_property(device_id, object_id, prop_id, lifetime_seconds, cov_increment, Box::new(|_| {}))
            .map_err(|e| format!("Failed to subscribe COV property: {}", e))?;

        tracing::info!(
            device_id,
            object_type,
            object_instance,
            property_id,
            process_id,
            "COV property subscription created"
        );

        Ok(process_id)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Cancel a COV subscription
#[tauri::command]
pub async fn unsubscribe_cov(
    process_id: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cov_manager = state.cov_manager.clone();

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let manager = cov_manager.lock().unwrap();
        let manager = manager.as_ref().ok_or("Service not initialized")?;

        manager
            .unsubscribe(process_id)
            .map_err(|e| format!("Failed to unsubscribe COV: {}", e))?;

        tracing::info!(process_id, "COV subscription cancelled");
        Ok(())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Poll for incoming COV notifications
#[tauri::command]
pub async fn poll_cov_notifications(
    timeout_ms: u64,
    state: State<'_, AppState>,
) -> Result<Vec<CovNotificationInfo>, String> {
    let cov_manager = state.cov_manager.clone();

    tokio::task::spawn_blocking(move || -> Result<Vec<CovNotificationInfo>, String> {
        let manager = cov_manager.lock().unwrap();
        let manager = manager.as_ref().ok_or("Service not initialized")?;

        let timeout = std::time::Duration::from_millis(timeout_ms);
        match manager.service().receive_cov_notification(timeout) {
            Ok(Some(notification)) => {
                let info = CovNotificationInfo {
                    device_id: notification.device_id,
                    subscriber_process_id: notification.subscriber_process_id,
                    object_type: notification.object_id.object_type.name().to_string(),
                    object_instance: notification.object_id.instance,
                    time_remaining: notification.time_remaining,
                    changed_values: notification
                        .changed_values
                        .iter()
                        .map(|(prop_id, value)| {
                            let (val_str, val_type) = property_value_to_string(value);
                            ChangedValueInfo {
                                property_id: format!("{:?}", prop_id),
                                value: val_str,
                                value_type: val_type,
                            }
                        })
                        .collect(),
                };
                Ok(vec![info])
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(format!("Failed to receive COV notification: {}", e)),
        }
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}
