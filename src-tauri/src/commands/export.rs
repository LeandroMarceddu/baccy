use crate::state::AppState;
use baccy_app::{parse_property_value, ObjectManager, PropertyManager};
use baccy_core::{
    DataType, ObjectId, ObjectType, PropertyId, PropertyValue,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedConfig {
    pub format_version: u32,
    pub export_timestamp: String,
    pub device_name: String,
    pub device_id: u32,
    pub device_address: String,
    pub objects: Vec<ExportedObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedObject {
    pub object_type: String,
    pub object_id: u32,
    pub object_name: String,
    pub properties: Vec<ExportedProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedProperty {
    pub property_id: String,
    pub property_name: String,
    pub value: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub total_objects: usize,
    pub total_properties: usize,
    pub successful_writes: usize,
    pub failed_writes: usize,
    pub errors: Vec<String>,
}

fn serialize_property_value(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Real(v) => format!("{:.2}", v),
        PropertyValue::Integer(v) => v.to_string(),
        PropertyValue::Unsigned(v) => v.to_string(),
        PropertyValue::Boolean(v) => v.to_string(),
        PropertyValue::String(v) => v.clone(),
        PropertyValue::Enumerated(v) => v.to_string(),
        PropertyValue::BitString(bits) => bits.iter().map(|b| if *b { '1' } else { '0' }).collect(),
        PropertyValue::ObjectIdentifier { object_type, instance } => {
            format!("{}:{}", object_type.name(), instance)
        }
    }
}

fn get_common_property_ids() -> Vec<PropertyId> {
    vec![
        PropertyId::ObjectName,
        PropertyId::PresentValue,
        PropertyId::Description,
        PropertyId::Units,
        PropertyId::StatusFlags,
        PropertyId::OutOfService,
        PropertyId::Reliability,
        PropertyId::EventState,
        PropertyId::Priority,
        PropertyId::VendorName,
        PropertyId::ModelName,
        PropertyId::FirmwareRevision,
        PropertyId::AppSoftwareRevision,
        PropertyId::ProtocolVersion,
        PropertyId::ProtocolRevision,
        PropertyId::Location,
        PropertyId::ProfileName,
        PropertyId::MaxApduLengthAccepted,
        PropertyId::SegmentationSupported,
        PropertyId::DeviceType,
        PropertyId::MaxSegmentsAccepted,
        PropertyId::MaxInfoFrames,
        PropertyId::ObjectType,
        PropertyId::ApduSegmentTimeout,
        PropertyId::ApduTimeout,
        PropertyId::ApduLength,
        PropertyId::LocalDate,
        PropertyId::LocalTime,
        PropertyId::DaylightSavingsStatus,
        PropertyId::TimeSynchronizationRecipients,
        PropertyId::TimeSynchronizationInterval,
        PropertyId::BackupAndRestoreState,
        PropertyId::BackupPreparationTime,
        PropertyId::RestorePreparationTime,
        PropertyId::RestoreCompletionTime,
        PropertyId::LastRestoreTime,
        PropertyId::ConfigurationFiles,
        PropertyId::DatabaseRevision,
        PropertyId::AckedTransitions,
        PropertyId::CovIncrement,
        PropertyId::TimeDelay,
        PropertyId::NotificationClass,
        PropertyId::EventEnable,
        PropertyId::EventDetectionEnable,
        PropertyId::EventAlgorithmInhibit,
        PropertyId::EventAlgorithmInhibitRef,
        PropertyId::EventAlarmInhibited,
        PropertyId::NotifyType,
        PropertyId::EventTimeStamps,
        PropertyId::EventMessageTexts,
        PropertyId::EventMessageTextsConfig,
        PropertyId::PriorityForWriting,
        PropertyId::AlarmValue,
        PropertyId::AlarmValues,
        PropertyId::FaultValues,
        PropertyId::Setpoint,
        PropertyId::SetpointReference,
        PropertyId::LogDeviceObjectProperty,
        PropertyId::LoggingType,
        PropertyId::LogInterval,
        PropertyId::LogObject,
        PropertyId::LoggingRecord,
        PropertyId::RecordsSinceNotification,
        PropertyId::LastNotifyRecord,
        PropertyId::NotificationThreshold,
        PropertyId::NotificationThresholdCount,
        PropertyId::BufferSize,
        PropertyId::RecordCount,
        PropertyId::TotalRecordCount,
        PropertyId::StartTime,
        PropertyId::StopTime,
        PropertyId::LogBuffer,
        PropertyId::Enable,
        PropertyId::NetworkNumber,
        PropertyId::NetworkNumberQuality,
        PropertyId::NetworkType,
        PropertyId::NetworkAccessSecurity,
        PropertyId::NetworkPriority,
        PropertyId::RoutingTable,
        PropertyId::RouterEntryDiscoveryTime,
        PropertyId::LinkSpeed,
        PropertyId::LinkSpeeds,
        PropertyId::LinkSpeedAutonegotiate,
        PropertyId::ProfileLocation,
        PropertyId::ValueSource,
        PropertyId::ValueSourceArray,
        PropertyId::ConstantValue,
        PropertyId::CommandTimeArray,
        PropertyId::DescriptionOfSchedule,
        PropertyId::PortLevel,
        PropertyId::PortNumber,
    ]
}

/// Export a device's entire configuration (all objects, all properties) as JSON.
#[tauri::command]
pub async fn export_device_config(
    device_id: u32,
    state: State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!(device_id, "Exporting device configuration");

    let service = {
        let lock = state.service.lock().unwrap();
        lock.as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    let device_name = {
        let dm = state.device_manager.lock().unwrap();
        dm.as_ref()
            .and_then(|m| m.get_device(device_id))
            .map(|d| d.name.clone())
            .unwrap_or_default()
    };

    let result = tokio::task::spawn_blocking(move || {
        // Load objects from the device
        let mut obj_manager = ObjectManager::new(Arc::clone(&service));
        obj_manager
            .load_objects(device_id)
            .map_err(|e| format!("Failed to load objects: {}", e))?;

        let objects: Vec<baccy_core::BacnetObject> =
            obj_manager.list_objects().into_iter().cloned().collect();

        let prop_ids = get_common_property_ids();
        let mut exported_objects = Vec::new();

        for obj in &objects {
            let mut prop_manager = PropertyManager::new(Arc::clone(&service));
            let object_id = ObjectId {
                object_type: obj.object_type,
                instance: obj.instance,
            };

            if let Err(e) = prop_manager.load_properties(device_id, object_id) {
                tracing::warn!(
                    device_id,
                    ?object_id,
                    error = %e,
                    "Failed to load properties for object, skipping"
                );
                continue;
            }

            let mut exported_props = Vec::new();
            for &pid in &prop_ids {
                if let Some(prop) = prop_manager.get_property(pid) {
                    exported_props.push(ExportedProperty {
                        property_id: format!("{:?}", prop.id),
                        property_name: prop.name.clone(),
                        value: serialize_property_value(&prop.value),
                        data_type: format!("{:?}", prop.data_type),
                    });
                }
            }

            exported_objects.push(ExportedObject {
                object_type: obj.object_type.name().to_string(),
                object_id: obj.instance,
                object_name: obj.name.clone(),
                properties: exported_props,
            });
        }

        let config = ExportedConfig {
            format_version: 1,
            export_timestamp: get_timestamp(),
            device_name,
            device_id,
            device_address: String::new(),
            objects: exported_objects,
        };

        serde_json::to_string_pretty(&config)
            .map_err(|e| format!("JSON serialization error: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Export failed: {}", e))?;

    tracing::info!(device_id, "Device configuration exported successfully");
    Ok(result)
}

fn parse_object_type(type_str: &str) -> Result<ObjectType, String> {
    ObjectType::from_display_name(type_str)
        .or_else(|| ObjectType::from_debug_name(type_str))
        .ok_or_else(|| format!("Unknown object type: {}", type_str))
}

fn parse_property_id(id_str: &str) -> Result<PropertyId, String> {
    let normalized = id_str.replace(' ', "");
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
        "VendorName" => Ok(PropertyId::VendorName),
        "ModelName" => Ok(PropertyId::ModelName),
        "FirmwareRevision" => Ok(PropertyId::FirmwareRevision),
        "AppSoftwareRevision" => Ok(PropertyId::AppSoftwareRevision),
        "ProtocolVersion" => Ok(PropertyId::ProtocolVersion),
        "ProtocolRevision" => Ok(PropertyId::ProtocolRevision),
        "Location" => Ok(PropertyId::Location),
        "ProfileName" => Ok(PropertyId::ProfileName),
        "SupportedObjectTypes" => Ok(PropertyId::SupportedObjectTypes),
        "ObjectList" => Ok(PropertyId::ObjectList),
        "PropertyList" => Ok(PropertyId::PropertyList),
        "MaxApduLengthAccepted" => Ok(PropertyId::MaxApduLengthAccepted),
        "SegmentationSupported" => Ok(PropertyId::SegmentationSupported),
        "DeviceAddressBinding" => Ok(PropertyId::DeviceAddressBinding),
        "DeviceType" => Ok(PropertyId::DeviceType),
        "MaxSegmentsAccepted" => Ok(PropertyId::MaxSegmentsAccepted),
        "MaxInfoFrames" => Ok(PropertyId::MaxInfoFrames),
        "ObjectType" => Ok(PropertyId::ObjectType),
        "ListOfObjectProperty" => Ok(PropertyId::ListOfObjectProperty),
        "ApduSegmentTimeout" => Ok(PropertyId::ApduSegmentTimeout),
        "ApduTimeout" => Ok(PropertyId::ApduTimeout),
        "ApduLength" => Ok(PropertyId::ApduLength),
        "LocalDate" => Ok(PropertyId::LocalDate),
        "LocalTime" => Ok(PropertyId::LocalTime),
        "DaylightSavingsStatus" => Ok(PropertyId::DaylightSavingsStatus),
        "TimeSynchronizationRecipients" => Ok(PropertyId::TimeSynchronizationRecipients),
        "TimeSynchronizationInterval" => Ok(PropertyId::TimeSynchronizationInterval),
        "BackupAndRestoreState" => Ok(PropertyId::BackupAndRestoreState),
        "BackupPreparationTime" => Ok(PropertyId::BackupPreparationTime),
        "RestorePreparationTime" => Ok(PropertyId::RestorePreparationTime),
        "RestoreCompletionTime" => Ok(PropertyId::RestoreCompletionTime),
        "LastRestoreTime" => Ok(PropertyId::LastRestoreTime),
        "ConfigurationFiles" => Ok(PropertyId::ConfigurationFiles),
        "DatabaseRevision" => Ok(PropertyId::DatabaseRevision),
        "AckedTransitions" => Ok(PropertyId::AckedTransitions),
        "CovIncrement" => Ok(PropertyId::CovIncrement),
        "TimeDelay" => Ok(PropertyId::TimeDelay),
        "NotificationClass" => Ok(PropertyId::NotificationClass),
        "EventEnable" => Ok(PropertyId::EventEnable),
        "EventDetectionEnable" => Ok(PropertyId::EventDetectionEnable),
        "EventAlgorithmInhibit" => Ok(PropertyId::EventAlgorithmInhibit),
        "EventAlgorithmInhibitRef" => Ok(PropertyId::EventAlgorithmInhibitRef),
        "EventAlarmInhibited" => Ok(PropertyId::EventAlarmInhibited),
        "NotifyType" => Ok(PropertyId::NotifyType),
        "EventTimeStamps" => Ok(PropertyId::EventTimeStamps),
        "EventMessageTexts" => Ok(PropertyId::EventMessageTexts),
        "EventMessageTextsConfig" => Ok(PropertyId::EventMessageTextsConfig),
        "PriorityForWriting" => Ok(PropertyId::PriorityForWriting),
        "AlarmValue" => Ok(PropertyId::AlarmValue),
        "AlarmValues" => Ok(PropertyId::AlarmValues),
        "FaultValues" => Ok(PropertyId::FaultValues),
        "Setpoint" => Ok(PropertyId::Setpoint),
        "SetpointReference" => Ok(PropertyId::SetpointReference),
        "LogDeviceObjectProperty" => Ok(PropertyId::LogDeviceObjectProperty),
        "LoggingType" => Ok(PropertyId::LoggingType),
        "LogInterval" => Ok(PropertyId::LogInterval),
        "LogObject" => Ok(PropertyId::LogObject),
        "LoggingRecord" => Ok(PropertyId::LoggingRecord),
        "RecordsSinceNotification" => Ok(PropertyId::RecordsSinceNotification),
        "LastNotifyRecord" => Ok(PropertyId::LastNotifyRecord),
        "NotificationThreshold" => Ok(PropertyId::NotificationThreshold),
        "NotificationThresholdCount" => Ok(PropertyId::NotificationThresholdCount),
        "BufferSize" => Ok(PropertyId::BufferSize),
        "RecordCount" => Ok(PropertyId::RecordCount),
        "TotalRecordCount" => Ok(PropertyId::TotalRecordCount),
        "StartTime" => Ok(PropertyId::StartTime),
        "StopTime" => Ok(PropertyId::StopTime),
        "LogBuffer" => Ok(PropertyId::LogBuffer),
        "Enable" => Ok(PropertyId::Enable),
        "NetworkNumber" => Ok(PropertyId::NetworkNumber),
        "NetworkNumberQuality" => Ok(PropertyId::NetworkNumberQuality),
        "NetworkType" => Ok(PropertyId::NetworkType),
        "NetworkAccessSecurity" => Ok(PropertyId::NetworkAccessSecurity),
        "NetworkPriority" => Ok(PropertyId::NetworkPriority),
        "RoutingTable" => Ok(PropertyId::RoutingTable),
        "RouterEntryDiscoveryTime" => Ok(PropertyId::RouterEntryDiscoveryTime),
        "LinkSpeed" => Ok(PropertyId::LinkSpeed),
        "LinkSpeeds" => Ok(PropertyId::LinkSpeeds),
        "LinkSpeedAutonegotiate" => Ok(PropertyId::LinkSpeedAutonegotiate),
        "StructuredObjectList" => Ok(PropertyId::StructuredObjectList),
        "SubordinateList" => Ok(PropertyId::SubordinateList),
        "SubordinateNodeTypes" => Ok(PropertyId::SubordinateNodeTypes),
        "SubordinateAnnotations" => Ok(PropertyId::SubordinateAnnotations),
        "SubordinateRelationships" => Ok(PropertyId::SubordinateRelationships),
        "SubordinateTags" => Ok(PropertyId::SubordinateTags),
        "ProfileLocation" => Ok(PropertyId::ProfileLocation),
        "ValueSource" => Ok(PropertyId::ValueSource),
        "ValueSourceArray" => Ok(PropertyId::ValueSourceArray),
        "ConstantValue" => Ok(PropertyId::ConstantValue),
        "CommandTimeArray" => Ok(PropertyId::CommandTimeArray),
        "DescriptionOfSchedule" => Ok(PropertyId::DescriptionOfSchedule),
        "PortLevel" => Ok(PropertyId::PortLevel),
        "PortNumber" => Ok(PropertyId::PortNumber),
        _ => Err(format!("Unknown property ID: {}", id_str)),
    }
}

fn parse_data_type(s: &str) -> Result<DataType, String> {
    match s {
        "Real" => Ok(DataType::Real),
        "Integer" => Ok(DataType::Integer),
        "Unsigned" => Ok(DataType::Unsigned),
        "Boolean" => Ok(DataType::Boolean),
        "CharacterString" => Ok(DataType::CharacterString),
        "Enumerated" => Ok(DataType::Enumerated),
        "BitString" => Ok(DataType::BitString),
        "ObjectIdentifier" => Ok(DataType::ObjectIdentifier),
        _ => Err(format!("Unknown data type: {}", s)),
    }
}

fn get_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let seconds = secs % 86400;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let sec = seconds % 60;

    let days = secs / 86400;
    let year = 1970_u64;
    let mut remaining_days = days;
    let mut y = year;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    let mut day = remaining_days + 1;
    for &md in &month_days {
        if day > md {
            day -= md;
            month += 1;
        } else {
            break;
        }
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, month, day, hours, minutes, sec, millis
    )
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Import a device configuration from JSON, writing properties to the device.
#[tauri::command]
pub async fn import_device_config(
    device_id: u32,
    config_json: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!(device_id, "Importing device configuration");

    let config: ExportedConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse configuration JSON: {}", e))?;

    let service = {
        let lock = state.service.lock().unwrap();
        lock.as_ref()
            .ok_or("BACnet service not initialized")?
            .clone()
    };

    let summary = tokio::task::spawn_blocking(move || {
        let mut total_properties = 0usize;
        let mut successful = 0usize;
        let mut failed = 0usize;
        let mut errors: Vec<String> = Vec::new();

        for obj in &config.objects {
            let obj_type = match parse_object_type(&obj.object_type) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(format!(
                        "Object {} ({}): {}",
                        obj.object_id, obj.object_type, e
                    ));
                    failed += 1;
                    continue;
                }
            };

            let object_id = ObjectId {
                object_type: obj_type,
                instance: obj.object_id,
            };

            for prop in &obj.properties {
                total_properties += 1;

                let prop_id = match parse_property_id(&prop.property_id) {
                    Ok(p) => p,
                    Err(e) => {
                        errors.push(format!(
                            "Object {} property '{}': {}",
                            obj.object_id, prop.property_id, e
                        ));
                        failed += 1;
                        continue;
                    }
                };

                let data_type = match parse_data_type(&prop.data_type) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(format!(
                            "Object {} property '{}': {}",
                            obj.object_id, prop.property_id, e
                        ));
                        failed += 1;
                        continue;
                    }
                };

                let value = match parse_property_value(&prop.value, data_type) {
                    Ok(v) => v,
                    Err(e) => {
                        errors.push(format!(
                            "Object {} property '{}': failed to parse value '{}': {}",
                            obj.object_id, prop.property_id, prop.value, e
                        ));
                        failed += 1;
                        continue;
                    }
                };

                match service.write_property(device_id, object_id, prop_id, value) {
                    Ok(_) => successful += 1,
                    Err(e) => {
                        errors.push(format!(
                            "Object {} property '{}': write failed: {}",
                            obj.object_id, prop.property_id, e
                        ));
                        failed += 1;
                    }
                }
            }
        }

        let summary = ImportSummary {
            total_objects: config.objects.len(),
            total_properties,
            successful_writes: successful,
            failed_writes: failed,
            errors,
        };

        serde_json::to_string(&summary)
            .map_err(|e| format!("Failed to serialize import summary: {}", e))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| format!("Import failed: {}", e))?;

    let summary_obj: ImportSummary = serde_json::from_str(&summary)
        .map_err(|e| format!("Failed to parse import summary: {}", e))?;

    tracing::info!(
        device_id,
        total_objects = summary_obj.total_objects,
        total_properties = summary_obj.total_properties,
        successful_writes = summary_obj.successful_writes,
        failed_writes = summary_obj.failed_writes,
        "Device configuration import completed"
    );

    Ok(summary)
}
