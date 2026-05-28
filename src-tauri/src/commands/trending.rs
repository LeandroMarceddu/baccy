// Trending commands for tracking property values over time

use crate::state::AppState;
use baccy_core::{ObjectId, ObjectType, PropertyId};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPointInfo {
    pub timestamp_ms: u64,
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendedPropertyInfo {
    pub device_id: u32,
    pub object_type: String,
    pub object_instance: u32,
    pub property_id: String,
    pub name: String,
    pub units: String,
    pub color: (u8, u8, u8),
    pub visible: bool,
    pub history: Vec<DataPointInfo>,
}

fn parse_object_type(type_str: &str) -> Result<ObjectType, String> {
    ObjectType::from_display_name(type_str)
        .or_else(|| ObjectType::from_debug_name(type_str))
        .ok_or_else(|| format!("Unknown object type: {}", type_str))
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

/// Add a property to trending
#[tauri::command]
pub async fn add_to_trending(
    device_id: u32,
    object_type: String,
    object_instance: u32,
    property_id: String,
    name: String,
    units: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(
        device_id,
        object_type,
        object_instance,
        property_id,
        "Adding property to trending"
    );
    
    let obj_type = parse_object_type(&object_type)?;
    let object_id = ObjectId {
        object_type: obj_type,
        instance: object_instance,
    };
    let prop_id = parse_property_id(&property_id)?;
    
    // Clone the trending manager Arc for the blocking task
    let trending_manager = state.trending_manager.clone();
    
    // Run add_property in blocking task since it does I/O
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut manager_guard = trending_manager.lock().unwrap();
        let manager = manager_guard
            .as_mut()
            .ok_or("Service not initialized")?;
        
        manager
            .add_property(device_id, object_id, prop_id, name, units)
            .map_err(|e| format!("Failed to add property to trending: {}", e))?;
        
        tracing::info!(
            device_id,
            object_type = ?object_id.object_type,
            object_instance = object_id.instance,
            property_id = ?prop_id,
            "Property added to trending successfully"
        );
        
        Ok(())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;
    
    Ok(())
}

/// Remove a property from trending
#[tauri::command]
pub fn remove_from_trending(index: usize, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!(index, "Removing property from trending");
    
    let mut trending_manager_guard = state.trending_manager.lock().unwrap();
    let trending_manager = trending_manager_guard
        .as_mut()
        .ok_or("Service not initialized")?;
    
    trending_manager.remove_property(index);
    
    tracing::info!(index, "Property removed from trending successfully");
    Ok(())
}

/// Get trending data for all properties
#[tauri::command]
pub fn get_trending_data(state: State<'_, AppState>) -> Result<Vec<TrendedPropertyInfo>, String> {
    let trending_manager_guard = state.trending_manager.lock().unwrap();
    let trending_manager = trending_manager_guard
        .as_ref()
        .ok_or("Service not initialized")?;
    
    // Get the epoch time for the first data point to calculate relative timestamps
    let base_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let properties: Vec<TrendedPropertyInfo> = trending_manager
        .properties()
        .iter()
        .map(|prop| {
            let history: Vec<DataPointInfo> = prop
                .history
                .iter()
                .map(|dp| {
                    // Convert Instant to milliseconds since epoch
                    // Note: This is approximate since Instant doesn't have a fixed epoch
                    let elapsed_ms = dp.timestamp.elapsed().as_millis() as u64;
                    let timestamp_ms = base_time.saturating_sub(elapsed_ms);
                    
                    DataPointInfo {
                        timestamp_ms,
                        value: dp.value,
                    }
                })
                .collect();
            
            TrendedPropertyInfo {
                device_id: prop.device_id,
                object_type: prop.object_id.object_type.name().to_string(),
                object_instance: prop.object_id.instance,
                property_id: format!("{:?}", prop.property_id),
                name: prop.name.clone(),
                units: prop.units.clone(),
                color: prop.color,
                visible: prop.visible,
                history,
            }
        })
        .collect();
    
    Ok(properties)
}

/// Export trending data as CSV
#[tauri::command]
pub fn export_trending_csv(state: State<'_, AppState>) -> Result<String, String> {
    let trending_manager_guard = state.trending_manager.lock().unwrap();
    let trending_manager = trending_manager_guard
        .as_ref()
        .ok_or("Service not initialized")?;

    let mut csv = String::from("Device,Object Type,Instance,Property,Name,Units,Timestamp,Value\n");
    for prop in trending_manager.properties().iter() {
        let obj_type = prop.object_id.object_type.name();
        for dp in &prop.history {
            let elapsed_ms = dp.timestamp.elapsed().as_millis() as u64;
            let base_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let ts = base_time.saturating_sub(elapsed_ms);
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                prop.device_id,
                obj_type,
                prop.object_id.instance,
                format!("{:?}", prop.property_id),
                prop.name,
                prop.units,
                ts,
                dp.value
            ));
        }
    }
    Ok(csv)
}

/// Export trending data as Parquet (returns base64-encoded bytes)
#[tauri::command]
pub fn export_trending_parquet(state: State<'_, AppState>) -> Result<String, String> {
    use arrow_array::{ArrayRef, Float32Array, Int32Array, RecordBatch, StringArray, UInt64Array};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::arrow_writer::ArrowWriter;
    use std::sync::Arc;

    let trending_manager_guard = state.trending_manager.lock().unwrap();
    let trending_manager = trending_manager_guard
        .as_ref()
        .ok_or("Service not initialized")?;

    let total_rows: usize = trending_manager
        .properties()
        .iter()
        .map(|p| p.history.len())
        .sum();

    if total_rows == 0 {
        return Err("No trending data to export".to_string());
    }

    let mut device_ids = Vec::with_capacity(total_rows);
    let mut obj_types = Vec::with_capacity(total_rows);
    let mut instances = Vec::with_capacity(total_rows);
    let mut prop_ids = Vec::with_capacity(total_rows);
    let mut names = Vec::with_capacity(total_rows);
    let mut units = Vec::with_capacity(total_rows);
    let mut timestamps = Vec::with_capacity(total_rows);
    let mut values = Vec::with_capacity(total_rows);

    for prop in trending_manager.properties().iter() {
        let obj_type = prop.object_id.object_type.name();
        for dp in &prop.history {
            let elapsed_ms = dp.timestamp.elapsed().as_millis() as u64;
            let base_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let ts = base_time.saturating_sub(elapsed_ms);

            device_ids.push(prop.device_id as i32);
            obj_types.push(obj_type.to_string());
            instances.push(prop.object_id.instance as i32);
            prop_ids.push(format!("{:?}", prop.property_id));
            names.push(prop.name.clone());
            units.push(prop.units.clone());
            timestamps.push(ts);
            values.push(dp.value);
        }
    }

    let schema = Arc::new(
        Schema::new(vec![
            Field::new("device_id", DataType::Int32, false),
            Field::new("object_type", DataType::Utf8, false),
            Field::new("instance", DataType::Int32, false),
            Field::new("property", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("units", DataType::Utf8, false),
            Field::new("timestamp_ms", DataType::UInt64, false),
            Field::new("value", DataType::Float32, false),
        ]),
    );

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(device_ids)) as ArrayRef,
            Arc::new(StringArray::from(obj_types)) as ArrayRef,
            Arc::new(Int32Array::from(instances)) as ArrayRef,
            Arc::new(StringArray::from(prop_ids)) as ArrayRef,
            Arc::new(StringArray::from(names)) as ArrayRef,
            Arc::new(StringArray::from(units)) as ArrayRef,
            Arc::new(UInt64Array::from(timestamps)) as ArrayRef,
            Arc::new(Float32Array::from(values)) as ArrayRef,
        ],
    )
    .map_err(|e| format!("Failed to create record batch: {e}"))?;

    let mut buf: Vec<u8> = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut buf, schema, None)
        .map_err(|e| format!("Failed to create writer: {e}"))?;
    writer
        .write(&batch)
        .map_err(|e| format!("Failed to write batch: {e}"))?;
    writer
        .close()
        .map_err(|e| format!("Failed to close writer: {e}"))?;

    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buf))
}

/// Clear all trending data
#[tauri::command]
pub fn clear_trending(state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Clearing all trending data");
    
    let mut trending_manager_guard = state.trending_manager.lock().unwrap();
    let trending_manager = trending_manager_guard
        .as_mut()
        .ok_or("Service not initialized")?;
    
    trending_manager.clear_all();
    
    tracing::info!("All trending data cleared successfully");
    Ok(())
}

/// Toggle visibility of a trended property
#[tauri::command]
pub fn toggle_trending_visibility(index: usize, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!(index, "Toggling trending visibility");
    
    let mut trending_manager_guard = state.trending_manager.lock().unwrap();
    let trending_manager = trending_manager_guard
        .as_mut()
        .ok_or("Service not initialized")?;
    
    trending_manager.toggle_visibility(index);
    
    tracing::info!(index, "Trending visibility toggled successfully");
    Ok(())
}

/// Poll all trended properties for new data
#[tauri::command]
pub async fn poll_trending(state: State<'_, AppState>) -> Result<(), String> {
    // Check if we should poll
    let should_poll = {
        let trending_manager_guard = state.trending_manager.lock().unwrap();
        let trending_manager = trending_manager_guard
            .as_ref()
            .ok_or("Service not initialized")?;
        trending_manager.should_poll()
    };
    
    if !should_poll {
        return Ok(());
    }
    
    // Clone the trending manager Arc for the blocking task
    let trending_manager = state.trending_manager.clone();
    
    // Run poll in blocking task
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut manager_guard = trending_manager.lock().unwrap();
        let manager = manager_guard
            .as_mut()
            .ok_or("Service not initialized")?;
        manager.poll()
            .map_err(|e| format!("Failed to poll: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;
    
    Ok(())
}
