use crate::state::AppState;
use baccy_core::{ObjectId, ObjectType, PropertyId, PropertyValue};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleProperty {
    pub id: String,
    pub name: String,
    pub value: String,
    pub readable: bool,
}

fn parse_object_type(type_str: &str) -> Result<ObjectType, String> {
    ObjectType::from_display_name(type_str)
        .or_else(|| ObjectType::from_debug_name(type_str))
        .ok_or_else(|| format!("Unknown object type: {}", type_str))
}

/// Read schedule-specific properties that PropertyValue can represent
#[tauri::command]
pub async fn read_schedule_data(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ScheduleProperty>, String> {
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId { object_type: obj_type, instance: object_instance };

    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock.as_ref().ok_or("BACnet service not initialized")?.clone()
    };

    let schedule_props: Vec<(PropertyId, &str, &str)> = vec![
        (PropertyId::PresentValue, "PresentValue", "Present Value"),
        (PropertyId::DescriptionOfSchedule, "DescriptionOfSchedule", "Description of Schedule"),
    ];

    tokio::task::spawn_blocking(move || -> Result<Vec<ScheduleProperty>, String> {
        let mut results = Vec::new();
        for (prop_id, id_str, name) in &schedule_props {
            match service.read_property(device_id, object_id, *prop_id) {
                Ok(val) => {
                    let display = format_property_value(&val);
                    results.push(ScheduleProperty {
                        id: id_str.to_string(),
                        name: name.to_string(),
                        value: display,
                        readable: true,
                    });
                }
                Err(e) => {
                    results.push(ScheduleProperty {
                        id: id_str.to_string(),
                        name: name.to_string(),
                        value: format!("{}", e),
                        readable: false,
                    });
                }
            }
        }
        Ok(results)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Read calendar-specific properties
#[tauri::command]
pub async fn read_calendar_data(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ScheduleProperty>, String> {
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId { object_type: obj_type, instance: object_instance };

    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock.as_ref().ok_or("BACnet service not initialized")?.clone()
    };

    let calendar_props: Vec<(PropertyId, &str, &str)> = vec![
        (PropertyId::PresentValue, "PresentValue", "Present Value"),
    ];

    tokio::task::spawn_blocking(move || -> Result<Vec<ScheduleProperty>, String> {
        let mut results = Vec::new();
        for (prop_id, id_str, name) in &calendar_props {
            match service.read_property(device_id, object_id, *prop_id) {
                Ok(val) => {
                    results.push(ScheduleProperty {
                        id: id_str.to_string(),
                        name: name.to_string(),
                        value: format_property_value(&val),
                        readable: true,
                    });
                }
                Err(e) => {
                    results.push(ScheduleProperty {
                        id: id_str.to_string(),
                        name: name.to_string(),
                        value: format!("{}", e),
                        readable: false,
                    });
                }
            }
        }
        Ok(results)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

fn format_property_value(v: &PropertyValue) -> String {
    match v {
        PropertyValue::Real(f) => format!("{:.4}", f),
        PropertyValue::Integer(i) => i.to_string(),
        PropertyValue::Unsigned(u) => u.to_string(),
        PropertyValue::Boolean(b) => b.to_string(),
        PropertyValue::String(s) => s.clone(),
        PropertyValue::Enumerated(e) => format!("Enumerated({})", e),
        PropertyValue::BitString(bits) => {
            bits.iter().map(|b| if *b { '1' } else { '0' }).collect()
        }
        PropertyValue::ObjectIdentifier { object_type, instance } => {
            format!("{}:{}", object_type.name(), instance)
        }
    }
}
