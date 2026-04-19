// Property reading and writing commands

use crate::state::AppState;
use baccy_core::{ObjectId, ObjectType, Property, PropertyId, PropertyValue};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub id: String,
    pub name: String,
    pub value: String,
    pub data_type: String,
    pub writable: bool,
    pub highlight_opacity: f32,
}

impl PropertyInfo {
    fn from_property(property: &Property, opacity: f32) -> Self {
        Self {
            id: format!("{:?}", property.id),
            name: property.name.clone(),
            value: format_property_value(&property.value),
            data_type: format!("{:?}", property.data_type),
            writable: property.writable,
            highlight_opacity: opacity,
        }
    }
}

fn format_property_value(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Real(v) => format!("{:.2}", v),
        PropertyValue::Integer(v) => v.to_string(),
        PropertyValue::Unsigned(v) => v.to_string(),
        PropertyValue::Boolean(v) => v.to_string(),
        PropertyValue::String(v) => v.clone(),
        PropertyValue::Enumerated(v) => v.to_string(),
        PropertyValue::BitString(bits) => {
            bits.iter()
                .map(|b| if *b { '1' } else { '0' })
                .collect()
        }
        PropertyValue::ObjectIdentifier { object_type, instance } => {
            format!("{}:{}", object_type.name(), instance)
        }
    }
}

fn parse_object_type(type_str: &str) -> Result<ObjectType, String> {
    match type_str {
        "Analog Input" => Ok(ObjectType::AnalogInput),
        "Analog Output" => Ok(ObjectType::AnalogOutput),
        "Analog Value" => Ok(ObjectType::AnalogValue),
        "Binary Input" => Ok(ObjectType::BinaryInput),
        "Binary Output" => Ok(ObjectType::BinaryOutput),
        "Binary Value" => Ok(ObjectType::BinaryValue),
        "Device" => Ok(ObjectType::Device),
        "Multi-State Input" => Ok(ObjectType::MultiStateInput),
        "Multi-State Output" => Ok(ObjectType::MultiStateOutput),
        "Multi-State Value" => Ok(ObjectType::MultiStateValue),
        _ => Err(format!("Unknown object type: {}", type_str)),
    }
}

fn parse_property_id(id_str: &str) -> Result<PropertyId, String> {
    // Remove spaces and handle both formats
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

/// Load properties for a device/object
#[tauri::command]
pub async fn load_properties(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PropertyInfo>, String> {
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        "Loading properties"
    );
    
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId {
        object_type: obj_type,
        instance: object_instance,
    };
    
    // Clone the service Arc for use in the blocking task
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };
    
    // Run load_properties in blocking task
    let properties = tokio::task::spawn_blocking(move || {
        let mut manager = baccy_app::PropertyManager::new(service);
        manager.load_properties(device_id, object_id)?;
        
        // Collect properties with their highlight opacity
        let props: Vec<(Property, f32)> = [
            PropertyId::ObjectName,
            PropertyId::PresentValue,
            PropertyId::Description,
            PropertyId::Units,
            PropertyId::StatusFlags,
        ]
        .iter()
        .filter_map(|&prop_id| {
            manager.get_property(prop_id).map(|prop| {
                let opacity = manager.get_highlight_opacity(prop_id);
                (prop.clone(), opacity)
            })
        })
        .collect();
        
        Ok::<Vec<(Property, f32)>, baccy_app::AppError>(props)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Failed to load properties: {}", e))?;
    
    let property_infos: Vec<PropertyInfo> = properties
        .iter()
        .map(|(prop, opacity)| PropertyInfo::from_property(prop, *opacity))
        .collect();
    
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        property_count = property_infos.len(),
        "Properties loaded successfully"
    );
    
    Ok(property_infos)
}

/// Get currently loaded properties
#[tauri::command]
pub fn get_properties(state: State<'_, AppState>) -> Result<Vec<PropertyInfo>, String> {
    let property_manager = state.property_manager.lock().unwrap();
    let manager = property_manager
        .as_ref()
        .ok_or("No properties loaded")?;
    
    let properties: Vec<PropertyInfo> = [
        PropertyId::ObjectName,
        PropertyId::PresentValue,
        PropertyId::Description,
        PropertyId::Units,
        PropertyId::StatusFlags,
    ]
    .iter()
    .filter_map(|&prop_id| {
        manager.get_property(prop_id).map(|prop| {
            let opacity = manager.get_highlight_opacity(prop_id);
            PropertyInfo::from_property(prop, opacity)
        })
    })
    .collect();
    
    Ok(properties)
}

/// Update a property value
#[tauri::command]
pub async fn update_property(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    property_id: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        property_id,
        value,
        "Updating property"
    );
    
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId {
        object_type: obj_type,
        instance: object_instance,
    };
    let prop_id = parse_property_id(&property_id)?;
    
    // Clone the service Arc for use in the blocking task
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };
    
    // Run update_property in blocking task
    tokio::task::spawn_blocking(move || {
        let mut manager = baccy_app::PropertyManager::new(service);
        // First load the properties to get the data type
        manager.load_properties(device_id, object_id)?;
        // Then update the property
        manager.update_property(device_id, object_id, prop_id, &value)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Failed to update property: {}", e))?;
    
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        property_id,
        "Property updated successfully"
    );
    
    Ok(())
}

/// Refresh properties for a device/object
#[tauri::command]
pub async fn refresh_properties(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PropertyInfo>, String> {
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        "Refreshing properties"
    );
    
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId {
        object_type: obj_type,
        instance: object_instance,
    };
    
    // Clone the service Arc for use in the blocking task
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };
    
    // Run refresh in blocking task
    let properties = tokio::task::spawn_blocking(move || {
        let mut manager = baccy_app::PropertyManager::new(service);
        manager.refresh(device_id, object_id)?;
        
        // Collect properties with their highlight opacity
        let props: Vec<(Property, f32)> = [
            PropertyId::ObjectName,
            PropertyId::PresentValue,
            PropertyId::Description,
            PropertyId::Units,
            PropertyId::StatusFlags,
        ]
        .iter()
        .filter_map(|&prop_id| {
            manager.get_property(prop_id).map(|prop| {
                let opacity = manager.get_highlight_opacity(prop_id);
                (prop.clone(), opacity)
            })
        })
        .collect();
        
        Ok::<Vec<(Property, f32)>, baccy_app::AppError>(props)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Failed to refresh properties: {}", e))?;
    
    let property_infos: Vec<PropertyInfo> = properties
        .iter()
        .map(|(prop, opacity)| PropertyInfo::from_property(prop, *opacity))
        .collect();
    
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        property_count = property_infos.len(),
        "Properties refreshed successfully"
    );
    
    Ok(property_infos)
}
