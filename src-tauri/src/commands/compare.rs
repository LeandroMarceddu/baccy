// Multi-object comparison command

use crate::state::AppState;
use baccy_core::{ObjectId, ObjectType, PropertyId, PropertyValue};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSelection {
    pub device_id: u32,
    pub object_type: String,
    pub instance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonObject {
    pub device_id: u32,
    pub device_name: String,
    pub object_type: String,
    pub instance: u32,
    pub object_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonProperty {
    pub property_name: String,
    pub values: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub objects: Vec<ComparisonObject>,
    pub properties: Vec<ComparisonProperty>,
}

fn parse_object_type(type_str: &str) -> Result<ObjectType, String> {
    ObjectType::from_display_name(type_str)
        .or_else(|| ObjectType::from_debug_name(type_str))
        .ok_or_else(|| format!("Unknown object type: {}", type_str))
}

fn format_value(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Real(v) => format!("{:.2}", v),
        PropertyValue::Integer(v) => v.to_string(),
        PropertyValue::Unsigned(v) => v.to_string(),
        PropertyValue::Boolean(v) => v.to_string(),
        PropertyValue::String(v) => v.clone(),
        PropertyValue::Enumerated(v) => v.to_string(),
        PropertyValue::BitString(bits) => {
            bits.iter().map(|b| if *b { '1' } else { '0' }).collect()
        }
        PropertyValue::ObjectIdentifier { object_type, instance } => {
            format!("{}:{}", object_type.name(), instance)
        }
    }
}

#[tauri::command]
pub async fn compare_objects(
    selections: Vec<ObjectSelection>,
    state: State<'_, AppState>,
) -> Result<ComparisonResult, String> {
    tracing::info!(count = selections.len(), "Comparing objects");

    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    let mut comp_objects: Vec<ComparisonObject> = Vec::new();
    // (property_name, values_per_object)
    let mut rows: Vec<(String, Vec<Option<String>>)> = Vec::new();

    for sel in &selections {
        let obj_type = parse_object_type(&sel.object_type)?;
        let object_id = ObjectId {
            object_type: obj_type,
            instance: sel.instance,
        };

        let device_name = {
            let dm = state.device_manager.lock().unwrap();
            dm.as_ref()
                .and_then(|m| m.get_device(sel.device_id))
                .map(|d| d.name.clone())
                .unwrap_or_default()
        };

        let service_clone = service.clone();
        let device_id = sel.device_id;
        let (props, obj_name) = tokio::task::spawn_blocking(move || {
            let mut manager = baccy_app::PropertyManager::new(service_clone);
            manager.load_properties(device_id, object_id)?;

            let mut obj_name = String::new();
            let props: Vec<(PropertyId, String)> = manager
                .list_properties()
                .into_iter()
                .map(|(id, prop)| {
                    let formatted = format_value(&prop.value);
                    if id == PropertyId::ObjectName {
                        obj_name = formatted.clone();
                    }
                    (id, formatted)
                })
                .collect();

            Ok::<_, baccy_app::AppError>((props, obj_name))
        })
        .await
        .map_err(|e| format!("Task error: {}", e))?
        .map_err(|e| format!("Failed to load properties: {}", e))?;

        comp_objects.push(ComparisonObject {
            device_id: sel.device_id,
            device_name,
            object_type: obj_type.name().to_string(),
            instance: sel.instance,
            object_name: obj_name,
        });

        for (prop_id, value_str) in &props {
            let prop_name = prop_id.name().to_string();
            match rows.iter_mut().find(|(name, _)| name.as_str() == prop_name) {
                Some((_, values)) => {
                    values.push(Some(value_str.clone()));
                }
                None => {
                    let mut values = Vec::new();
                    for _ in 0..comp_objects.len() - 1 {
                        values.push(None);
                    }
                    values.push(Some(value_str.clone()));
                    rows.push((prop_name.clone(), values));
                }
            }
        }

        // Ensure every row has an entry for this object
        for (_, values) in &mut rows {
            if values.len() < comp_objects.len() {
                values.push(None);
            }
        }
    }

    let properties: Vec<ComparisonProperty> = rows
        .into_iter()
        .map(|(property_name, values)| ComparisonProperty {
            property_name,
            values,
        })
        .collect();

    tracing::info!(
        object_count = comp_objects.len(),
        property_count = properties.len(),
        "Comparison complete"
    );

    Ok(ComparisonResult {
        objects: comp_objects,
        properties,
    })
}
