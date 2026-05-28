use baccy_core::{Address, Device, DeviceId, ObjectId, ObjectType, PropertyId, PropertyValue};
use baccy_transport::network_stats::StatsCollector;
use baccy_transport::{Transport, TransportError};
use std::collections::HashMap;

use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use bacnet_rs;

pub mod device_state;
pub mod request_throttle;
pub use request_throttle::RequestThrottle;

use bacnet_rs::object::property_identifier::PropertyIdentifier;
use bacnet_rs::object::ObjectIdentifier;
use bacnet_rs::service::{ConfirmedServiceChoice, UnconfirmedServiceChoice};

/// Result of an I-Have response
#[derive(Debug, Clone)]
pub struct IHaveInfo {
    pub device_id: DeviceId,
    pub object_id: ObjectId,
    pub object_name: String,
    pub source_address: Address,
}

/// Parsed event notification (confirmed or unconfirmed)
#[derive(Debug, Clone)]
pub struct EventNotificationInfo {
    pub initiating_device: DeviceId,
    pub event_object: ObjectId,
    pub timestamp: Option<String>,
    pub notification_class: u32,
    pub priority: u32,
    pub event_type: u32,
    pub notify_type: u32,
    pub ack_required: bool,
    pub event_state: u32,
}

/// Response to GetEventInformation
#[derive(Debug, Clone)]
pub struct GetEventInformationResponse {
    pub summaries: Vec<EventSummaryInfo>,
    pub more_events: bool,
}

/// A single event summary from GetEventInformation
#[derive(Debug, Clone)]
pub struct EventSummaryInfo {
    pub event_object: ObjectId,
    pub event_state: u32,
    pub acknowledged_transitions: Vec<bool>,
    pub notify_type: u32,
    pub event_enable: Vec<bool>,
    pub event_priorities: Vec<u32>,
}

/// Response from ConfirmedEventNotification — one per requested object
#[derive(Debug, Clone)]
pub struct EventNotificationResponse {
    pub event_object: ObjectId,
    pub event_state: u32,
    pub event_type: u32,
    pub notify_type: u32,
    pub event_enable: Vec<bool>,
    pub event_priorities: Vec<u32>,
    pub ack_required: bool,
    pub event_time_stamps: Vec<Option<String>>,
    pub event_message_text: Option<String>,
    pub optional_context: Option<String>,
    pub local_timestamp: Option<String>,
}

/// Response from AcknowledgeAlarm
#[derive(Debug, Clone)]
pub struct AcknowledgeAlarmResponse {
    pub acknowledged_state_changed: bool,
    pub acked_transitions: Vec<bool>,
    pub acked_transitions_time: Vec<Option<String>>,
}

/// Per-device state for managing confirmed requests
#[derive(Clone)]
struct DeviceState {
    address: Address,
    invoke_counter: u8,
    max_apdu: bacnet_rs::app::MaxApduSize,
    seg_supported: bacnet_rs::object::Segmentation,
}

impl DeviceState {
    fn next_invoke_id(&mut self) -> u8 {
        let id = self.invoke_counter;
        self.invoke_counter = self.invoke_counter.wrapping_add(1);
        id
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 2000,
            backoff_multiplier: 2.0,
        }
    }
}

pub mod type_conversion {
    use baccy_core::{ObjectType, PropertyId, PropertyValue};
    use bacnet_rs::object::property_identifier::PropertyIdentifier;
    use bacnet_rs::object::ObjectType as BacnetObjectType;

    pub fn to_bacnet_object_type(obj_type: ObjectType) -> BacnetObjectType {
        match obj_type {
            ObjectType::AnalogInput => BacnetObjectType::AnalogInput,
            ObjectType::AnalogOutput => BacnetObjectType::AnalogOutput,
            ObjectType::AnalogValue => BacnetObjectType::AnalogValue,
            ObjectType::BinaryInput => BacnetObjectType::BinaryInput,
            ObjectType::BinaryOutput => BacnetObjectType::BinaryOutput,
            ObjectType::BinaryValue => BacnetObjectType::BinaryValue,
            ObjectType::Device => BacnetObjectType::Device,
            ObjectType::MultiStateInput => BacnetObjectType::MultiStateInput,
            ObjectType::MultiStateOutput => BacnetObjectType::MultiStateOutput,
            ObjectType::MultiStateValue => BacnetObjectType::MultiStateValue,
            ObjectType::Calendar => BacnetObjectType::Calendar,
            ObjectType::Command => BacnetObjectType::Command,
            ObjectType::File => BacnetObjectType::File,
            ObjectType::Group => BacnetObjectType::Group,
            ObjectType::EventEnrollment => BacnetObjectType::EventEnrollment,
            ObjectType::Program => BacnetObjectType::Program,
            ObjectType::Schedule => BacnetObjectType::Schedule,
            ObjectType::Averaging => BacnetObjectType::Averaging,
            ObjectType::NotificationClass => BacnetObjectType::NotificationClass,
            ObjectType::TrendLog => BacnetObjectType::TrendLog,
            ObjectType::LifeSafetyPoint => BacnetObjectType::LifeSafetyPoint,
            ObjectType::LifeSafetyZone => BacnetObjectType::LifeSafetyZone,
            ObjectType::Loop => BacnetObjectType::Loop,
            ObjectType::Accumulator => BacnetObjectType::Accumulator,
            ObjectType::PulseConverter => BacnetObjectType::PulseConverter,
            ObjectType::EventLog => BacnetObjectType::EventLog,
            ObjectType::StructuredView => BacnetObjectType::StructuredView,
            ObjectType::AccessDoor => BacnetObjectType::AccessDoor,
            ObjectType::CredentialDataInput => BacnetObjectType::CredentialDataInput,
            ObjectType::BitStringValue => BacnetObjectType::BitstringValue,
            ObjectType::CharacterStringValue => BacnetObjectType::CharacterstringValue,
            ObjectType::DateTimeValue => BacnetObjectType::DatetimeValue,
            ObjectType::OctetStringValue => BacnetObjectType::OctetstringValue,
            ObjectType::IntegerValue => BacnetObjectType::IntegerValue,
            ObjectType::PositiveIntegerValue => BacnetObjectType::PositiveIntegerValue,
            ObjectType::TimeValue => BacnetObjectType::TimeValue,
            ObjectType::NotificationForwarder => BacnetObjectType::NotificationForwarder,
            ObjectType::NetworkPort => BacnetObjectType::NetworkPort,
            ObjectType::ElevatorGroup => BacnetObjectType::ElevatorGroup,
            ObjectType::Escalator => BacnetObjectType::Escalator,
            ObjectType::Timer => BacnetObjectType::Timer,
            ObjectType::NetworkSecurity => {
                // Not officially in BACnet standard 135-2020; use a custom value
                BacnetObjectType::from(128u32)
            }
            ObjectType::AlarmGroup => {
                // Not officially in BACnet standard 135-2020; use a custom value
                BacnetObjectType::from(129u32)
            }
        }
    }

    pub fn from_bacnet_object_type(obj_type: BacnetObjectType) -> Option<ObjectType> {
        match obj_type {
            BacnetObjectType::AnalogInput => Some(ObjectType::AnalogInput),
            BacnetObjectType::AnalogOutput => Some(ObjectType::AnalogOutput),
            BacnetObjectType::AnalogValue => Some(ObjectType::AnalogValue),
            BacnetObjectType::BinaryInput => Some(ObjectType::BinaryInput),
            BacnetObjectType::BinaryOutput => Some(ObjectType::BinaryOutput),
            BacnetObjectType::BinaryValue => Some(ObjectType::BinaryValue),
            BacnetObjectType::Device => Some(ObjectType::Device),
            BacnetObjectType::MultiStateInput => Some(ObjectType::MultiStateInput),
            BacnetObjectType::MultiStateOutput => Some(ObjectType::MultiStateOutput),
            BacnetObjectType::MultiStateValue => Some(ObjectType::MultiStateValue),
            BacnetObjectType::Calendar => Some(ObjectType::Calendar),
            BacnetObjectType::Command => Some(ObjectType::Command),
            BacnetObjectType::File => Some(ObjectType::File),
            BacnetObjectType::Group => Some(ObjectType::Group),
            BacnetObjectType::EventEnrollment => Some(ObjectType::EventEnrollment),
            BacnetObjectType::Program => Some(ObjectType::Program),
            BacnetObjectType::Schedule => Some(ObjectType::Schedule),
            BacnetObjectType::Averaging => Some(ObjectType::Averaging),
            BacnetObjectType::NotificationClass => Some(ObjectType::NotificationClass),
            BacnetObjectType::TrendLog => Some(ObjectType::TrendLog),
            BacnetObjectType::LifeSafetyPoint => Some(ObjectType::LifeSafetyPoint),
            BacnetObjectType::LifeSafetyZone => Some(ObjectType::LifeSafetyZone),
            BacnetObjectType::Loop => Some(ObjectType::Loop),
            BacnetObjectType::Accumulator => Some(ObjectType::Accumulator),
            BacnetObjectType::PulseConverter => Some(ObjectType::PulseConverter),
            BacnetObjectType::EventLog => Some(ObjectType::EventLog),
            BacnetObjectType::StructuredView => Some(ObjectType::StructuredView),
            BacnetObjectType::AccessDoor => Some(ObjectType::AccessDoor),
            BacnetObjectType::CredentialDataInput => Some(ObjectType::CredentialDataInput),
            BacnetObjectType::BitstringValue => Some(ObjectType::BitStringValue),
            BacnetObjectType::CharacterstringValue => Some(ObjectType::CharacterStringValue),
            BacnetObjectType::DatetimeValue => Some(ObjectType::DateTimeValue),
            BacnetObjectType::IntegerValue => Some(ObjectType::IntegerValue),
            BacnetObjectType::OctetstringValue => Some(ObjectType::OctetStringValue),
            BacnetObjectType::PositiveIntegerValue => Some(ObjectType::PositiveIntegerValue),
            BacnetObjectType::TimeValue => Some(ObjectType::TimeValue),
            BacnetObjectType::NotificationForwarder => Some(ObjectType::NotificationForwarder),
            BacnetObjectType::NetworkPort => Some(ObjectType::NetworkPort),
            BacnetObjectType::ElevatorGroup => Some(ObjectType::ElevatorGroup),
            BacnetObjectType::Escalator => Some(ObjectType::Escalator),
            BacnetObjectType::Timer => Some(ObjectType::Timer),
            _ => {
                if bacnet_rs::object::ObjectType::from(128u32) == obj_type {
                    Some(ObjectType::NetworkSecurity)
                } else if bacnet_rs::object::ObjectType::from(129u32) == obj_type {
                    Some(ObjectType::AlarmGroup)
                } else {
                    None
                }
            }
        }
    }

    pub fn to_bacnet_property_id(prop_id: PropertyId) -> PropertyIdentifier {
        match prop_id {
            // Core types
            PropertyId::PresentValue => PropertyIdentifier::PresentValue,
            PropertyId::ObjectName => PropertyIdentifier::ObjectName,
            PropertyId::Description => PropertyIdentifier::Description,
            PropertyId::Units => PropertyIdentifier::Units,
            PropertyId::StatusFlags => PropertyIdentifier::StatusFlags,
            PropertyId::OutOfService => PropertyIdentifier::OutOfService,
            PropertyId::Reliability => PropertyIdentifier::Reliability,
            PropertyId::EventState => PropertyIdentifier::EventState,
            PropertyId::Priority => PropertyIdentifier::Priority,
            // Device/Vendor
            PropertyId::VendorName => PropertyIdentifier::VendorName,
            PropertyId::ModelName => PropertyIdentifier::ModelName,
            PropertyId::FirmwareRevision => PropertyIdentifier::FirmwareRevision,
            PropertyId::AppSoftwareRevision => PropertyIdentifier::ApplicationSoftwareVersion,
            PropertyId::ProtocolVersion => PropertyIdentifier::ProtocolVersion,
            PropertyId::ProtocolRevision => PropertyIdentifier::ProtocolRevision,
            PropertyId::Location => PropertyIdentifier::Location,
            PropertyId::ProfileName => PropertyIdentifier::ProfileName,
            // Lists/Capabilities
            PropertyId::SupportedObjectTypes => PropertyIdentifier::from(37u32),
            PropertyId::ObjectList => PropertyIdentifier::ObjectList,
            PropertyId::PropertyList => PropertyIdentifier::PropertyList,
            PropertyId::MaxApduLengthAccepted => PropertyIdentifier::MaxApduLengthAccepted,
            PropertyId::SegmentationSupported => PropertyIdentifier::SegmentationSupported,
            PropertyId::DeviceAddressBinding => PropertyIdentifier::DeviceAddressBinding,
            PropertyId::DeviceType => PropertyIdentifier::DeviceType,
            PropertyId::MaxSegmentsAccepted => PropertyIdentifier::MaxSegmentsAccepted,
            PropertyId::MaxInfoFrames => PropertyIdentifier::MaxInfoFrames,
            PropertyId::ObjectType => PropertyIdentifier::ObjectType,
            PropertyId::ListOfObjectProperty => PropertyIdentifier::ListOfObjectPropertyReferences,
            // APDU/Timeout
            PropertyId::ApduSegmentTimeout => PropertyIdentifier::ApduSegmentTimeout,
            PropertyId::ApduTimeout => PropertyIdentifier::ApduTimeout,
            PropertyId::ApduLength => PropertyIdentifier::ApduLength,
            // TimeSync
            PropertyId::LocalDate => PropertyIdentifier::LocalDate,
            PropertyId::LocalTime => PropertyIdentifier::LocalTime,
            PropertyId::DaylightSavingsStatus => PropertyIdentifier::DaylightSavingsStatus,
            PropertyId::TimeSynchronizationRecipients => {
                PropertyIdentifier::TimeSynchronizationRecipients
            }
            PropertyId::TimeSynchronizationInterval => {
                PropertyIdentifier::TimeSynchronizationInterval
            }
            // Backup/Restore
            PropertyId::BackupAndRestoreState => PropertyIdentifier::BackupAndRestoreState,
            PropertyId::BackupPreparationTime => PropertyIdentifier::BackupPreparationTime,
            PropertyId::RestorePreparationTime => PropertyIdentifier::RestorePreparationTime,
            PropertyId::RestoreCompletionTime => PropertyIdentifier::RestoreCompletionTime,
            PropertyId::LastRestoreTime => PropertyIdentifier::LastRestoreTime,
            PropertyId::ConfigurationFiles => PropertyIdentifier::ConfigurationFiles,
            PropertyId::DatabaseRevision => PropertyIdentifier::DatabaseRevision,
            PropertyId::ActiveCovSubscriptions => PropertyIdentifier::ActiveCovSubscriptions,
            PropertyId::ActiveCovMultipleSubscriptions => {
                PropertyIdentifier::ActiveCovMultipleSubscriptions
            }
            // Alarming/Event
            PropertyId::AckedTransitions => PropertyIdentifier::AckedTransitions,
            PropertyId::CovIncrement => PropertyIdentifier::CovIncrement,
            PropertyId::TimeDelay => PropertyIdentifier::TimeDelay,
            PropertyId::NotificationClass => PropertyIdentifier::NotificationClass,
            PropertyId::EventEnable => PropertyIdentifier::EventEnable,
            PropertyId::EventDetectionEnable => PropertyIdentifier::EventDetectionEnable,
            PropertyId::EventAlgorithmInhibit => PropertyIdentifier::EventAlgorithmInhibit,
            PropertyId::EventAlgorithmInhibitRef => PropertyIdentifier::EventAlgorithmInhibitRef,
            PropertyId::EventAlarmInhibited => PropertyIdentifier::from(607u32),
            PropertyId::NotifyType => PropertyIdentifier::NotifyType,
            PropertyId::EventTimeStamps => PropertyIdentifier::EventTimeStamps,
            PropertyId::EventMessageTexts => PropertyIdentifier::EventMessageTexts,
            PropertyId::EventMessageTextsConfig => PropertyIdentifier::EventMessageTextsConfig,
            PropertyId::PriorityForWriting => PropertyIdentifier::PriorityForWriting,
            PropertyId::AlarmValue => PropertyIdentifier::AlarmValue,
            PropertyId::AlarmValues => PropertyIdentifier::AlarmValues,
            PropertyId::FaultValues => PropertyIdentifier::FaultValues,
            PropertyId::Setpoint => PropertyIdentifier::Setpoint,
            PropertyId::SetpointReference => PropertyIdentifier::SetpointReference,
            // Trending/Logging
            PropertyId::LogDeviceObjectProperty => PropertyIdentifier::LogDeviceObjectProperty,
            PropertyId::LoggingType => PropertyIdentifier::LoggingType,
            PropertyId::LogInterval => PropertyIdentifier::LogInterval,
            PropertyId::LogObject => PropertyIdentifier::LoggingObject,
            PropertyId::LoggingRecord => PropertyIdentifier::LoggingRecord,
            PropertyId::RecordsSinceNotification => PropertyIdentifier::RecordsSinceNotification,
            PropertyId::LastNotifyRecord => PropertyIdentifier::LastNotifyRecord,
            PropertyId::NotificationThreshold => PropertyIdentifier::NotificationThreshold,
            PropertyId::NotificationThresholdCount => PropertyIdentifier::from(608u32),
            PropertyId::BufferSize => PropertyIdentifier::BufferSize,
            PropertyId::RecordCount => PropertyIdentifier::RecordCount,
            PropertyId::TotalRecordCount => PropertyIdentifier::TotalRecordCount,
            PropertyId::StartTime => PropertyIdentifier::StartTime,
            PropertyId::StopTime => PropertyIdentifier::StopTime,
            PropertyId::LogBuffer => PropertyIdentifier::LogBuffer,
            PropertyId::Enable => PropertyIdentifier::Enable,
            // Network
            PropertyId::NetworkNumber => PropertyIdentifier::NetworkNumber,
            PropertyId::NetworkNumberQuality => PropertyIdentifier::NetworkNumberQuality,
            PropertyId::NetworkType => PropertyIdentifier::NetworkType,
            PropertyId::NetworkAccessSecurity => PropertyIdentifier::from(600u32),
            PropertyId::NetworkPriority => PropertyIdentifier::from(601u32),
            PropertyId::RoutingTable => PropertyIdentifier::RoutingTable,
            PropertyId::RouterEntryDiscoveryTime => PropertyIdentifier::from(602u32),
            PropertyId::LinkSpeed => PropertyIdentifier::LinkSpeed,
            PropertyId::LinkSpeeds => PropertyIdentifier::LinkSpeeds,
            PropertyId::LinkSpeedAutonegotiate => PropertyIdentifier::LinkSpeedAutonegotiate,
            // StructuredView
            PropertyId::StructuredObjectList => PropertyIdentifier::StructuredObjectList,
            PropertyId::SubordinateList => PropertyIdentifier::SubordinateList,
            PropertyId::SubordinateNodeTypes => PropertyIdentifier::SubordinateNodeTypes,
            PropertyId::SubordinateAnnotations => PropertyIdentifier::SubordinateAnnotations,
            PropertyId::SubordinateRelationships => PropertyIdentifier::SubordinateRelationships,
            PropertyId::SubordinateTags => PropertyIdentifier::SubordinateTags,
            // Other
            PropertyId::ProfileLocation => PropertyIdentifier::ProfileLocation,
            PropertyId::ValueSource => PropertyIdentifier::ValueSource,
            PropertyId::ValueSourceArray => PropertyIdentifier::ValueSourceArray,
            PropertyId::ConstantValue => PropertyIdentifier::from(605u32),
            PropertyId::CommandTimeArray => PropertyIdentifier::CommandTimeArray,
            PropertyId::DescriptionOfSchedule => PropertyIdentifier::from(606u32),
            PropertyId::PortLevel => PropertyIdentifier::from(603u32),
            PropertyId::PortNumber => PropertyIdentifier::from(604u32),
        }
    }

    pub fn to_bacnet_value(value: PropertyValue) -> Result<Vec<u8>, String> {
        use bacnet_rs::encoding::advanced::bitstring::encode_bit_string;
        use bacnet_rs::encoding::{
            encode_boolean, encode_character_string, encode_enumerated, encode_object_identifier,
            encode_real, encode_signed, encode_unsigned,
        };

        let mut buffer = Vec::new();
        match value {
            PropertyValue::Real(f) => {
                encode_real(&mut buffer, f).map_err(|e| format!("Failed to encode Real: {}", e))?
            }
            PropertyValue::Integer(i) => encode_signed(&mut buffer, i as i32)
                .map_err(|e| format!("Failed to encode Integer: {}", e))?,
            PropertyValue::Unsigned(u) => encode_unsigned(&mut buffer, u as u32)
                .map_err(|e| format!("Failed to encode Unsigned: {}", e))?,
            PropertyValue::Boolean(b) => encode_boolean(&mut buffer, b)
                .map_err(|e| format!("Failed to encode Boolean: {}", e))?,
            PropertyValue::String(s) => encode_character_string(&mut buffer, &s)
                .map_err(|e| format!("Failed to encode String: {}", e))?,
            PropertyValue::Enumerated(e) => {
                encode_enumerated(&mut buffer, e);
            }
            PropertyValue::BitString(bits) => encode_bit_string(&mut buffer, &bits)
                .map_err(|e| format!("Failed to encode BitString: {}", e))?,
            PropertyValue::ObjectIdentifier {
                object_type,
                instance,
            } => {
                let bacnet_obj_id = bacnet_rs::object::ObjectIdentifier::new(
                    to_bacnet_object_type(object_type),
                    instance,
                );
                encode_object_identifier(&mut buffer, bacnet_obj_id)
                    .map_err(|e| format!("Failed to encode ObjectIdentifier: {}", e))?;
            }
        }
        Ok(buffer)
    }

    pub fn from_bacnet_value(data: &[u8]) -> Result<PropertyValue, String> {
        use bacnet_rs::encoding::advanced::bitstring::decode_bit_string;
        use bacnet_rs::encoding::{
            decode_boolean, decode_character_string, decode_enumerated, decode_object_identifier,
            decode_real, decode_signed, decode_unsigned,
        };

        if data.is_empty() {
            return Err("Empty data buffer".to_string());
        }

        let tag_number = (data[0] >> 4) & 0x0F;
        match tag_number {
            1 => {
                let (value, _) =
                    decode_boolean(data).map_err(|e| format!("Failed to decode Boolean: {}", e))?;
                Ok(PropertyValue::Boolean(value))
            }
            2 => {
                let (value, _) = decode_unsigned(data)
                    .map_err(|e| format!("Failed to decode Unsigned: {}", e))?;
                Ok(PropertyValue::Unsigned(value as u64))
            }
            3 => {
                let (value, _) =
                    decode_signed(data).map_err(|e| format!("Failed to decode Integer: {}", e))?;
                Ok(PropertyValue::Integer(value as i64))
            }
            4 => {
                let (value, _) =
                    decode_real(data).map_err(|e| format!("Failed to decode Real: {}", e))?;
                Ok(PropertyValue::Real(value))
            }
            7 => {
                let (value, _) = decode_character_string(data)
                    .map_err(|e| format!("Failed to decode String: {}", e))?;
                Ok(PropertyValue::String(value))
            }
            8 => {
                let (value, _) = decode_bit_string(data)
                    .map_err(|e| format!("Failed to decode BitString: {}", e))?;
                Ok(PropertyValue::BitString(value))
            }
            9 => {
                let (value, _) = decode_enumerated(data)
                    .map_err(|e| format!("Failed to decode Enumerated: {}", e))?;
                Ok(PropertyValue::Enumerated(value))
            }
            12 => {
                let (obj_id, _) = decode_object_identifier(data)
                    .map_err(|e| format!("Failed to decode ObjectIdentifier: {}", e))?;
                let object_type = from_bacnet_object_type(obj_id.object_type)
                    .ok_or_else(|| format!("Unsupported object type: {:?}", obj_id.object_type))?;
                Ok(PropertyValue::ObjectIdentifier {
                    object_type,
                    instance: obj_id.instance,
                })
            }
            _ => Err(format!("Unsupported application tag: {}", tag_number)),
        }
    }

    pub fn convert_bacnet_property_value(
        value: &bacnet_rs::property::PropertyValue,
    ) -> Result<PropertyValue, String> {
        use bacnet_rs::property::PropertyValue as BacnetPropertyValue;

        match value {
            BacnetPropertyValue::Null => Err("Null property values are not supported".to_string()),
            BacnetPropertyValue::Boolean(b) => Ok(PropertyValue::Boolean(*b)),
            BacnetPropertyValue::Unsigned(u) => Ok(PropertyValue::Unsigned(*u)),
            BacnetPropertyValue::Signed(i) => Ok(PropertyValue::Integer(*i)),
            BacnetPropertyValue::Real(f) => Ok(PropertyValue::Real(*f)),
            BacnetPropertyValue::Double(d) => Ok(PropertyValue::Real(*d as f32)),
            BacnetPropertyValue::OctetString(bytes) => {
                let hex_string = bytes
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                Ok(PropertyValue::String(hex_string))
            }
            BacnetPropertyValue::CharacterString(s) => Ok(PropertyValue::String(s.clone())),
            BacnetPropertyValue::BitString(bits) => Ok(PropertyValue::BitString(bits.clone())),
            BacnetPropertyValue::Enumerated(e) => Ok(PropertyValue::Enumerated(*e)),
            BacnetPropertyValue::Date(month, day, year, _weekday) => {
                let date_str = format!(
                    "{:04}-{:02}-{:02}",
                    if *year == 255 { 0 } else { 1900 + *year as u32 },
                    if *month == 255 { 0 } else { *month as u32 },
                    if *day == 255 { 0 } else { *day as u32 }
                );
                Ok(PropertyValue::String(date_str))
            }
            BacnetPropertyValue::Time(hour, minute, second, hundredths) => {
                let time_str = format!(
                    "{:02}:{:02}:{:02}.{:02}",
                    if *hour == 255 { 0 } else { *hour },
                    if *minute == 255 { 0 } else { *minute },
                    if *second == 255 { 0 } else { *second },
                    if *hundredths == 255 { 0 } else { *hundredths }
                );
                Ok(PropertyValue::String(time_str))
            }
            BacnetPropertyValue::ObjectIdentifier(obj_id) => {
                let object_type = from_bacnet_object_type(obj_id.object_type)
                    .ok_or_else(|| format!("Unsupported object type: {:?}", obj_id.object_type))?;
                Ok(PropertyValue::ObjectIdentifier {
                    object_type,
                    instance: obj_id.instance,
                })
            }
            BacnetPropertyValue::Unknown(_) => {
                Err("Unknown property value type is not supported".to_string())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("BACnet error: class={class:?}, code={code:?}")]
    BacnetError { class: ErrorClass, code: ErrorCode },

    #[error("Operation timed out")]
    Timeout,
}

impl ProtocolError {
    pub fn user_message(&self) -> String {
        match self {
            ProtocolError::TransportError(e) => e.user_message(),
            ProtocolError::EncodingError(msg) => format!("Failed to encode BACnet message: {}", msg),
            ProtocolError::DecodingError(msg) => format!("Failed to decode BACnet response: {}", msg),
            ProtocolError::BacnetError { class, code } => {
                format!("BACnet error: {} - {}", class.description(), code.description())
            }
            ProtocolError::Timeout => {
                "Operation timed out waiting for device response. The device may be offline or unreachable.".to_string()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Device,
    Object,
    Property,
    Resources,
    Security,
    Services,
    Vt,
    Communication,
    Unknown(u8),
}

impl ErrorClass {
    pub fn description(&self) -> &'static str {
        match self {
            ErrorClass::Device => "Device Error",
            ErrorClass::Object => "Object Error",
            ErrorClass::Property => "Property Error",
            ErrorClass::Resources => "Resources Error",
            ErrorClass::Security => "Security Error",
            ErrorClass::Services => "Services Error",
            ErrorClass::Vt => "Virtual Terminal Error",
            ErrorClass::Communication => "Communication Error",
            ErrorClass::Unknown(_) => "Unknown Error Class",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Other,
    UnknownObject,
    UnknownProperty,
    WriteAccessDenied,
    ReadAccessDenied,
    InvalidDataType,
    ValueOutOfRange,
    Timeout,
    DeviceBusy,
    Unknown(u8),
}

impl ErrorCode {
    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::Other => "Other error",
            ErrorCode::UnknownObject => "Unknown object",
            ErrorCode::UnknownProperty => "Unknown property",
            ErrorCode::WriteAccessDenied => "Write access denied",
            ErrorCode::ReadAccessDenied => "Read access denied",
            ErrorCode::InvalidDataType => "Invalid data type",
            ErrorCode::ValueOutOfRange => "Value out of range",
            ErrorCode::Timeout => "Timeout",
            ErrorCode::DeviceBusy => "Device is busy",
            ErrorCode::Unknown(_) => "Unknown error code",
        }
    }
}

fn format_object_id(object: &ObjectId) -> String {
    format!("{:?}({})", object.object_type, object.instance)
}

fn format_property_id(property: &PropertyId) -> String {
    format!("{:?}", property)
}

fn map_error_class(class: u8) -> ErrorClass {
    match class {
        0 => ErrorClass::Device,
        1 => ErrorClass::Object,
        2 => ErrorClass::Property,
        3 => ErrorClass::Resources,
        4 => ErrorClass::Security,
        5 => ErrorClass::Services,
        6 => ErrorClass::Vt,
        7 => ErrorClass::Communication,
        _ => ErrorClass::Unknown(class),
    }
}

fn map_error_code(code: u8) -> ErrorCode {
    match code {
        0 => ErrorCode::Other,
        31 => ErrorCode::UnknownObject,
        32 => ErrorCode::UnknownProperty,
        36 => ErrorCode::ReadAccessDenied,
        37 => ErrorCode::WriteAccessDenied,
        38 => ErrorCode::InvalidDataType,
        40 => ErrorCode::ValueOutOfRange,
        41 => ErrorCode::DeviceBusy,
        _ => ErrorCode::Unknown(code),
    }
}

/// Encode a ConfirmedRequest APDU and send it, segmenting if the service_data
/// exceeds the device's max APDU size.
fn send_confirmed_request(
    transport: &dyn Transport,
    address: &Address,
    service_choice: ConfirmedServiceChoice,
    service_data: &[u8],
    invoke_id: u8,
    seg_accepted: bool,
    max_apdu: bacnet_rs::app::MaxApduSize,
    stats: &StatsCollector,
) -> Result<(), ProtocolError> {
    use bacnet_rs::app::{Apdu, MaxSegments};
    use std::time::Duration;

    // Reserve 24 bytes for BVLC (4) + NPDU (~4) + APDU header (~16 for segmented)
    const HEADER_OVERHEAD: usize = 24;
    let max_apdu_size = max_apdu.size();
    let max_payload_per_segment = max_apdu_size.saturating_sub(HEADER_OVERHEAD);

    if service_data.len() <= max_payload_per_segment {
        // Fits in one segment — send as before (unchanged)
        let apdu = Apdu::ConfirmedRequest {
            segmented: false,
            more_follows: false,
            segmented_response_accepted: seg_accepted,
            max_segments: MaxSegments::Unspecified,
            max_response_size: max_apdu,
            invoke_id,
            sequence_number: None,
            proposed_window_size: None,
            service_choice,
            service_data: service_data.to_vec(),
        };

        let mut npdu = bacnet_rs::network::Npdu::new();
        npdu.control.expecting_reply = true;
        let mut message = npdu.encode();
        message.extend_from_slice(&apdu.encode());
        stats.record_send(message.len());
        let result = transport.send(address, &message);
        if result.is_err() {
            stats.record_error();
        }
        Ok(result?)
    } else {
        // Need to segment — send in multiple parts, one at a time, waiting for SegmentAck
        let window_size = 1;
        let mut sequence_number: u8 = 0;
        let total_payload = service_data.len();
        let mut offset = 0;

        while offset < total_payload {
            let remaining = total_payload - offset;
            let segment_size = std::cmp::min(remaining, max_payload_per_segment);
            let segment = &service_data[offset..offset + segment_size];
            let is_last = offset + segment_size >= total_payload;
            let more_follows = !is_last;

            let apdu = Apdu::ConfirmedRequest {
                segmented: true,
                more_follows,
                segmented_response_accepted: seg_accepted,
                max_segments: MaxSegments::Unspecified,
                max_response_size: max_apdu,
                invoke_id,
                sequence_number: Some(sequence_number),
                proposed_window_size: Some(window_size),
                service_choice,
                service_data: segment.to_vec(),
            };

            let mut npdu = bacnet_rs::network::Npdu::new();
            npdu.control.expecting_reply = more_follows;
            let mut message = npdu.encode();
            message.extend_from_slice(&apdu.encode());
            stats.record_send(message.len());
            if transport.send(address, &message).is_err() {
                stats.record_error();
            }

            // Wait for SegmentAck before sending next segment
            if more_follows {
                let ack_timeout = Duration::from_millis(3000);
                receive_segment_ack(transport, address, invoke_id, sequence_number, ack_timeout)?;
            }

            offset += segment_size;
            sequence_number = sequence_number.wrapping_add(1);
        }

        Ok(())
    }
}

/// Wait for a SegmentAck confirming receipt of a segment.
fn receive_segment_ack(
    transport: &dyn Transport,
    address: &Address,
    invoke_id: u8,
    expected_seq: u8,
    timeout: Duration,
) -> Result<bacnet_rs::app::Apdu, ProtocolError> {
    use bacnet_rs::app::Apdu;

    let start = std::time::Instant::now();
    loop {
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return Err(ProtocolError::Timeout);
        }
        let remaining = timeout - elapsed;

        let (resp_addr, data) = transport.receive(remaining)?;
        if resp_addr != *address {
            continue;
        }

        let mut offset = 0;
        if let Ok((_, npdu_len)) = bacnet_rs::network::Npdu::decode(&data[offset..]) {
            offset += npdu_len;
        } else {
            continue;
        }

        if offset >= data.len() {
            continue;
        }

        if let Ok(apdu) = Apdu::decode(&data[offset..]) {
            match apdu {
                Apdu::SegmentAck {
                    invoke_id: resp_invoke,
                    sequence_number: seq,
                    ..
                } if resp_invoke == invoke_id && seq == expected_seq => {
                    return Ok(apdu);
                }
                Apdu::Abort { .. } => {
                    return Err(ProtocolError::DecodingError("Segment aborted".to_string()));
                }
                _ => continue,
            }
        }
    }
}

/// Receive a response APDU, handling segmentation reassembly.
/// Returns the fully reassembled service_data and service_choice.
fn receive_response(
    transport: &dyn Transport,
    address: &Address,
    invoke_id: u8,
    expected_service: ConfirmedServiceChoice,
    timeout: Duration,
    stats: &StatsCollector,
) -> Result<(Vec<u8>, ConfirmedServiceChoice), ProtocolError> {
    use bacnet_rs::app::Apdu;
    use bacnet_rs::network::Npdu;

    let start_time = std::time::Instant::now();

    // Segmented response reassembly buffer
    let mut seg_buffer: Option<(u8, Vec<Vec<u8>>)> = None;

    loop {
        let elapsed = start_time.elapsed();
        let remaining = timeout.checked_sub(elapsed).unwrap_or(Duration::ZERO);
        if remaining.is_zero() {
            return Err(ProtocolError::Timeout);
        }

        let (response_address, response_bytes) = match transport.receive(remaining) {
            Ok(result) => {
                stats.record_receive(result.1.len());
                result
            }
            Err(e) => {
                stats.record_error();
                return Err(e.into());
            }
        };
        if response_address != *address {
            continue;
        }

        let mut offset = 0;
        let (_, npdu_len) = match Npdu::decode(&response_bytes[offset..]) {
            Ok(v) => v,
            Err(_) => continue,
        };
        offset += npdu_len;

        if offset >= response_bytes.len() {
            continue;
        }

        let apdu = match Apdu::decode(&response_bytes[offset..]) {
            Ok(a) => a,
            Err(_) => continue,
        };

        match apdu {
            Apdu::ComplexAck {
                invoke_id: resp_invoke,
                service_choice,
                service_data,
                segmented,
                more_follows,
                sequence_number,
                ..
            } => {
                if resp_invoke != invoke_id {
                    continue;
                }
                if service_choice != expected_service {
                    continue;
                }

                if !segmented && !more_follows {
                    return Ok((service_data, service_choice));
                }

                // Segmented response — accumulate
                let seq = sequence_number.unwrap_or(0);
                if !more_follows {
                    // Final segment
                    match &mut seg_buffer {
                        Some((_, segments)) => {
                            segments.push(service_data);
                            let mut all_data = Vec::new();
                            for seg in segments.drain(..) {
                                all_data.extend(seg);
                            }
                            return Ok((all_data, service_choice));
                        }
                        None => return Ok((service_data, service_choice)),
                    }
                } else {
                    // More segments follow
                    match &mut seg_buffer {
                        Some((last_seq, segments)) => {
                            if seq as u16 != *last_seq as u16 + 1 {
                                return Err(ProtocolError::DecodingError(
                                    "Segmented response out of order".to_string(),
                                ));
                            }
                            segments.push(service_data);
                            *last_seq = seq;
                        }
                        None => {
                            seg_buffer = Some((seq, vec![service_data]));
                        }
                    }
                }
            }
            Apdu::SimpleAck { service_choice, .. } => {
                if service_choice == expected_service as u8 {
                    return Ok((Vec::new(), expected_service));
                }
            }
            Apdu::Error {
                service_choice,
                error_class,
                error_code,
                ..
            } => {
                if service_choice == expected_service {
                    return Err(ProtocolError::BacnetError {
                        class: map_error_class(error_class),
                        code: map_error_code(error_code),
                    });
                }
            }
            Apdu::Reject { reject_reason, .. } => {
                return Err(ProtocolError::DecodingError(format!(
                    "Request rejected: {:?}",
                    reject_reason
                )));
            }
            Apdu::Abort { abort_reason, .. } => {
                return Err(ProtocolError::DecodingError(format!(
                    "Request aborted: {:?}",
                    abort_reason
                )));
            }
            _ => continue,
        }
    }
}

/// Parse an I-Am message, extract device details and MaxApduSize.
pub fn parse_iam(data: &[u8], _source_address: Address) -> Result<Device, ProtocolError> {
    use bacnet_rs::network::Npdu;
    use bacnet_rs::service::IAmRequest;

    let mut offset = 0;
    if data.len() >= 4 && data[0] == 0x81 {
        offset = 4;
    }

    if data.len() <= offset {
        return Err(ProtocolError::DecodingError(
            "Message too short".to_string(),
        ));
    }

    let (_, npdu_len) = Npdu::decode(&data[offset..])
        .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode NPDU: {}", e)))?;
    offset += npdu_len;

    if data.len() <= offset || data[offset] != 0x10 {
        return Err(ProtocolError::DecodingError(
            "Invalid I-Am message".to_string(),
        ));
    }
    offset += 2;

    let iam = IAmRequest::decode(&data[offset..])
        .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode I-Am: {}", e)))?;

    Ok(Device {
        instance: iam.device_identifier.instance,
        name: format!("Device {}", iam.device_identifier.instance),
        vendor_id: iam.vendor_identifier,
        vendor_name: String::new(),
        model_name: String::new(),
        description: String::new(),
    })
}

/// Extract MaxApduSize from an I-Am message payload (bytes after APDU header).
/// Returns (max_apdu_size_enum, segmentation_supported_enum).
pub fn parse_max_apdu_from_iam(
    data: &[u8],
) -> (bacnet_rs::app::MaxApduSize, bacnet_rs::object::Segmentation) {
    // I-Am payload: ObjectId(4) + MaxApduLength(1) + Segmentation(1) + VendorId(2/3)
    // This is a best-effort parse; returns defaults on failure.
    if data.len() < 8 {
        return (
            bacnet_rs::app::MaxApduSize::Up1476,
            bacnet_rs::object::Segmentation::NoSegmentation,
        );
    }
    // MaxApduLength is a u8 enum at offset 5 (after objectId + first tag byte)
    // We need to find the MaxApduLength and Segmentation values by scanning application tags
    let max_apdu = match data.get(5).copied().unwrap_or(5) {
        0 => bacnet_rs::app::MaxApduSize::Up50,
        1 => bacnet_rs::app::MaxApduSize::Up128,
        2 => bacnet_rs::app::MaxApduSize::Up206,
        3 => bacnet_rs::app::MaxApduSize::Up480,
        4 => bacnet_rs::app::MaxApduSize::Up1024,
        5 => bacnet_rs::app::MaxApduSize::Up1476,
        _ => bacnet_rs::app::MaxApduSize::Up1476,
    };

    let seg = match data.get(6).copied().unwrap_or(3) {
        0 => bacnet_rs::object::Segmentation::NoSegmentation,
        1 => bacnet_rs::object::Segmentation::Both,
        2 => bacnet_rs::object::Segmentation::Transmit,
        3 => bacnet_rs::object::Segmentation::Receive,
        _ => bacnet_rs::object::Segmentation::NoSegmentation,
    };

    (max_apdu, seg)
}

/// Configuration for per-device request throttling
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    pub max_concurrent_per_device: usize,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            max_concurrent_per_device: 1,
        }
    }
}

/// BACnet service for protocol operations
pub struct BacnetService {
    transport: Arc<dyn Transport>,
    request_timeout: Duration,
    devices: Arc<Mutex<HashMap<DeviceId, DeviceState>>>,
    stats: Arc<StatsCollector>,
    retry_config: RetryConfig,
    device_tracker: Arc<device_state::DeviceTracker>,
    request_throttle: RequestThrottle,
}

impl BacnetService {
    pub fn new(transport: Arc<dyn Transport>, timeout: Duration) -> Self {
        Self::with_config(
            transport,
            timeout,
            Arc::new(StatsCollector::new()),
            RetryConfig::default(),
            ThrottleConfig::default(),
        )
    }

    pub fn with_stats(
        transport: Arc<dyn Transport>,
        timeout: Duration,
        stats: Arc<StatsCollector>,
    ) -> Self {
        Self::with_config(
            transport,
            timeout,
            stats,
            RetryConfig::default(),
            ThrottleConfig::default(),
        )
    }

    pub fn with_config(
        transport: Arc<dyn Transport>,
        timeout: Duration,
        stats: Arc<StatsCollector>,
        retry_config: RetryConfig,
        throttle_config: ThrottleConfig,
    ) -> Self {
        const MIN_TIMEOUT: Duration = Duration::from_millis(100);
        const MAX_TIMEOUT: Duration = Duration::from_secs(30);

        if timeout < MIN_TIMEOUT || timeout > MAX_TIMEOUT {
            panic!("Timeout must be between 100ms and 30 seconds");
        }

        Self {
            transport,
            request_timeout: timeout,
            devices: Arc::new(Mutex::new(HashMap::new())),
            stats,
            retry_config,
            device_tracker: Arc::new(device_state::DeviceTracker::new(3)),
            request_throttle: RequestThrottle::new(throttle_config.max_concurrent_per_device),
        }
    }

    pub fn get_transport(&self) -> Arc<dyn Transport> {
        Arc::clone(&self.transport)
    }

    pub fn get_stats(&self) -> Arc<StatsCollector> {
        Arc::clone(&self.stats)
    }

    pub fn throttle(&self) -> &RequestThrottle {
        &self.request_throttle
    }

    pub fn get_device_tracker(&self) -> &Arc<device_state::DeviceTracker> {
        &self.device_tracker
    }

    pub fn get_retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    /// Wrap an operation with per-device throttle acquire/release
    fn with_throttle<R>(
        &self,
        device_id: DeviceId,
        operation: impl FnOnce() -> Result<R, ProtocolError>,
    ) -> Result<R, ProtocolError> {
        self.request_throttle.acquire(device_id);
        let result = operation();
        self.request_throttle.release(device_id);
        result
    }

    fn get_device_state_mut<F, R>(&self, device_id: DeviceId, f: F) -> Result<R, ProtocolError>
    where
        F: FnOnce(&mut DeviceState) -> R,
    {
        let mut devices = self.devices.lock().unwrap();
        devices.get_mut(&device_id).map(|s| f(s)).ok_or_else(|| {
            ProtocolError::DecodingError(format!(
                "Unknown device {}: Device must be discovered via Who-Is/I-Am first",
                device_id
            ))
        })
    }

    fn get_device_address(&self, device_id: DeviceId) -> Result<Address, ProtocolError> {
        let devices = self.devices.lock().unwrap();
        devices
            .get(&device_id)
            .map(|s| s.address.clone())
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!(
                    "Unknown device {}: Device must be discovered via Who-Is/I-Am first",
                    device_id
                ))
            })
    }

    /// Cache device address and its capabilities from I-Am
    pub fn cache_device(
        &self,
        device_id: DeviceId,
        address: Address,
        max_apdu: bacnet_rs::app::MaxApduSize,
        seg_supported: bacnet_rs::object::Segmentation,
    ) {
        let mut devices = self.devices.lock().unwrap();
        devices.insert(
            device_id,
            DeviceState {
                address,
                invoke_counter: 1,
                max_apdu,
                seg_supported,
            },
        );
    }

    pub fn who_is(&self) -> Result<(), ProtocolError> {
        use bacnet_rs::app::Apdu;
        use bacnet_rs::network::Npdu;
        use bacnet_rs::service::WhoIsRequest;

        let who_is = WhoIsRequest::new();
        let mut service_data = Vec::new();
        who_is
            .encode(&mut service_data)
            .map_err(|e| ProtocolError::EncodingError(format!("Failed to encode Who-Is: {}", e)))?;

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: UnconfirmedServiceChoice::WhoIs,
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        self.stats.record_send(message.len());
        if self.transport.broadcast(&message).is_err() {
            self.stats.record_error();
        }
        Ok(())
    }

    pub fn who_is_range(&self, low: u32, high: u32) -> Result<(), ProtocolError> {
        use bacnet_rs::app::Apdu;
        use bacnet_rs::network::Npdu;
        use bacnet_rs::service::WhoIsRequest;

        let who_is = WhoIsRequest::for_range(low, high);
        let mut service_data = Vec::new();
        who_is.encode(&mut service_data).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode Who-Is range: {}", e))
        })?;

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: UnconfirmedServiceChoice::WhoIs,
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        self.stats.record_send(message.len());
        if self.transport.broadcast(&message).is_err() {
            self.stats.record_error();
        }
        Ok(())
    }

    /// Send a Who-Has request to find an object by name.
    pub fn who_has_by_name(&self, object_name: &str) -> Result<(), ProtocolError> {
        use bacnet_rs::app::Apdu;
        use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
        use bacnet_rs::encoding::encode_character_string;
        use bacnet_rs::network::Npdu;

        let mut service_data = Vec::new();

        // Per BACnet standard: object [1] CharacterString
        // The CharacterString is an application-tagged type wrapped in context tag 1,
        // so we use opening/closing tags.
        encode_opening_tag(&mut service_data, 1).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode opening tag: {}", e))
        })?;

        encode_character_string(&mut service_data, object_name).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode object name: {}", e))
        })?;

        encode_closing_tag(&mut service_data, 1).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode closing tag: {}", e))
        })?;

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: UnconfirmedServiceChoice::WhoHas,
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        self.transport.broadcast(&message)?;
        Ok(())
    }

    /// Send a Who-Has request to find an object by object identifier.
    pub fn who_has_by_object(
        &self,
        object_type: ObjectType,
        instance: u32,
    ) -> Result<(), ProtocolError> {
        use crate::type_conversion::to_bacnet_object_type;
        use bacnet_rs::app::Apdu;
        use bacnet_rs::encoding::encode_context_object_id;
        use bacnet_rs::network::Npdu;
        use bacnet_rs::object::ObjectIdentifier;

        let mut service_data = Vec::new();

        let bacnet_obj_type = to_bacnet_object_type(object_type);
        let obj_id = ObjectIdentifier::new(bacnet_obj_type, instance);

        // Per BACnet standard: object [2] BACnetObjectIdentifier
        // ObjectIdentifier can be encoded with a direct context tag (primitive encoding).
        let encoded = encode_context_object_id(obj_id, 2).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode object id: {}", e))
        })?;
        service_data.extend_from_slice(&encoded);

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: UnconfirmedServiceChoice::WhoHas,
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        self.stats.record_send(message.len());
        self.transport.broadcast(&message)?;
        Ok(())
    }

    /// Receive an I-Have response (unconfirmed). Blocks with timeout.
    pub fn receive_ihave(&self, timeout: Duration) -> Result<IHaveInfo, ProtocolError> {
        use crate::type_conversion::from_bacnet_object_type;
        use bacnet_rs::encoding::{decode_character_string, decode_context_object_id};
        use bacnet_rs::network::Npdu;

        loop {
            let (source_address, data) = match self.transport.receive(timeout) {
                Ok(result) => {
                    self.stats.record_receive(result.1.len());
                    result
                }
                Err(e) => {
                    self.stats.record_error();
                    return Err(e.into());
                }
            };

            let mut offset = 0;
            if data.len() >= 4 && data[0] == 0x81 {
                offset = 4;
            }

            if data.len() <= offset {
                continue;
            }
            let (_, npdu_len) = match Npdu::decode(&data[offset..]) {
                Ok(v) => v,
                Err(_) => continue,
            };
            offset += npdu_len;

            if data.len() <= offset + 1 {
                continue;
            }
            if data[offset] != 0x10 {
                continue;
            }
            if data[offset + 1] != 1 {
                continue;
            }
            offset += 2;

            let payload = &data[offset..];
            let mut pos = 0;

            let (dev_obj_id, consumed) = decode_context_object_id(payload, 0).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode device id: {}", e))
            })?;
            pos += consumed;

            let (obj_obj_id, consumed) =
                decode_context_object_id(&payload[pos..], 1).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode object id: {}", e))
                })?;
            pos += consumed;

            if pos >= payload.len() {
                return Err(ProtocolError::DecodingError(
                    "Unexpected end of I-Have payload".to_string(),
                ));
            }
            // object_name [2] CharacterString — constructed encoding with opening/closing tags
            let tag_byte = payload[pos];
            let is_opening_tag2 = (tag_byte & 0x0F) == 0x0E && (tag_byte >> 4) == 2;
            if !is_opening_tag2 {
                return Err(ProtocolError::DecodingError(format!(
                    "Expected opening tag 2 for object name, got {:02x}",
                    tag_byte
                )));
            }
            pos += 1; // skip opening tag 2

            let (object_name, consumed) =
                decode_character_string(&payload[pos..]).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode object name: {}", e))
                })?;
            pos += consumed;

            // Skip closing tag 2 if present (no-op but validates message structure)
            if pos < payload.len() {
                let close_byte = payload[pos];
                let close_tag = close_byte >> 4;
                let is_closing = (close_byte & 0x0F) == 0x0F;
                if is_closing && close_tag == 2 {
                    // consumed
                }
            }

            let device_id = dev_obj_id.instance;
            let object_type = from_bacnet_object_type(obj_obj_id.object_type).ok_or_else(|| {
                ProtocolError::DecodingError(format!(
                    "Unknown BACnet object type: {:?}",
                    obj_obj_id.object_type
                ))
            })?;

            return Ok(IHaveInfo {
                device_id,
                object_id: ObjectId {
                    object_type,
                    instance: obj_obj_id.instance,
                },
                object_name,
                source_address,
            });
        }
    }

    /// Receive and parse I-Am responses, caching address + capabilities
    pub fn receive_iam(&self, timeout: Duration) -> Result<Device, ProtocolError> {
        use bacnet_rs::network::Npdu;

        let (source_address, data) = match self.transport.receive(timeout) {
            Ok(result) => {
                self.stats.record_receive(result.1.len());
                result
            }
            Err(e) => {
                self.stats.record_error();
                return Err(e.into());
            }
        };

        // Parse MaxApduSize and segmentation from the raw message
        let mut offset = 0;
        if data.len() >= 4 && data[0] == 0x81 {
            offset = 4;
        }
        if offset < data.len() {
            if let Ok((_, npdu_len)) = Npdu::decode(&data[offset..]) {
                offset += npdu_len;
            }
        }
        // Skip APDU header (type + service = 2 bytes)
        let payload_offset = offset + 2;

        let (max_apdu, seg_supported) = parse_max_apdu_from_iam(&data[payload_offset..]);

        let device = parse_iam(&data, source_address.clone())?;

        // Cache device with capabilities
        self.cache_device(device.instance, source_address, max_apdu, seg_supported);

        Ok(device)
    }

    /// Read a property's raw response values from a BACnet object.
    /// Returns all decoded property values (for array-type properties, this may be >1).
    pub fn read_property_raw(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
    ) -> Result<Vec<bacnet_rs::property::PropertyValue>, ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::object::ObjectIdentifier;
            use bacnet_rs::service::ReadPropertyRequest;
            use type_conversion::{to_bacnet_object_type, to_bacnet_property_id};

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id =
                ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);
            let bacnet_property_id = to_bacnet_property_id(property);

            let read_request = ReadPropertyRequest::new(bacnet_object_id, bacnet_property_id);
            let mut service_data = Vec::new();
            read_request.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode ReadProperty: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::ReadProperty,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            let (response_data, _) = receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::ReadProperty,
                self.request_timeout,
                &self.stats,
            )?;

            let response = bacnet_rs::service::ReadPropertyResponse::decode(&response_data)
                .map_err(|e| {
                    ProtocolError::DecodingError(format!(
                    "Failed to parse property read response for {} property {} from device {}: {}.",
                    format_object_id(&object), format_property_id(&property), device, e
                ))
                })?;

            Ok(response.property_values)
        })
    }

    /// Read a property from a BACnet object using dynamic invoke_id
    pub fn read_property(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
    ) -> Result<PropertyValue, ProtocolError> {
        let values = self.read_property_raw(device, object, property)?;

        if values.is_empty() {
            return Err(ProtocolError::DecodingError(format!(
                "Property read returned no values for {} property {} from device {}.",
                format_object_id(&object),
                format_property_id(&property),
                device
            )));
        }

        let property_value =
            type_conversion::convert_bacnet_property_value(&values[0]).map_err(|e| {
                ProtocolError::DecodingError(format!(
                    "Failed to convert property value for {} property {} from device {}: {}.",
                    format_object_id(&object),
                    format_property_id(&property),
                    device,
                    e
                ))
            })?;

        Ok(property_value)
    }

    /// Try to read the PropertyList property from an object.
    ///
    /// PropertyList (property ID 111) returns a list of property identifiers
    /// that the object supports. Returns `Ok(Some(list))` on success,
    /// `Ok(None)` if the property is not supported by the object,
    /// and `Err(e)` for actual communication errors.
    pub fn read_property_list(
        &self,
        device: DeviceId,
        object: ObjectId,
    ) -> Result<Option<Vec<PropertyId>>, ProtocolError> {
        match self.read_property_raw(device, object, PropertyId::PropertyList) {
            Ok(values) => {
                let mut property_ids = Vec::new();
                for value in &values {
                    if let bacnet_rs::property::PropertyValue::Enumerated(enum_val) = value {
                        let our_id = map_unsigned_to_property_id(*enum_val);
                        // Only include PresentValue if the raw enum value is actually 85,
                        // since map_unsigned_to_property_id uses PresentValue as a fallback for unknown properties
                        if our_id != PropertyId::PresentValue || *enum_val == 85 {
                            property_ids.push(our_id);
                        }
                    }
                }
                Ok(Some(property_ids))
            }
            Err(ProtocolError::BacnetError {
                code: ErrorCode::UnknownProperty,
                ..
            })
            | Err(ProtocolError::DecodingError(_)) => {
                // PropertyList not supported by this object, or response couldn't be decoded
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Read a property array element
    pub fn read_property_array(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        _index: u32,
    ) -> Result<PropertyValue, ProtocolError> {
        self.read_property(device, object, property)
    }

    /// Write a property value
    pub fn write_property(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        value: PropertyValue,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::object::ObjectIdentifier;
            use bacnet_rs::service::WritePropertyRequest;
            use type_conversion::{to_bacnet_object_type, to_bacnet_property_id, to_bacnet_value};

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id =
                ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);
            let bacnet_property_id = to_bacnet_property_id(property);
            let property_value_bytes = to_bacnet_value(value).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode value: {}", e))
            })?;

            let write_request = WritePropertyRequest::new(
                bacnet_object_id,
                bacnet_property_id.into(),
                property_value_bytes,
            );

            let mut service_data = Vec::new();
            write_request.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode WriteProperty: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::WriteProperty,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::WriteProperty,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    /// Write a property array element
    pub fn write_property_array(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        _index: u32,
        value: PropertyValue,
    ) -> Result<(), ProtocolError> {
        self.write_property(device, object, property, value)
    }

    /// Write a property with priority
    pub fn write_property_priority(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        value: PropertyValue,
        priority: u8,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::object::ObjectIdentifier;
            use bacnet_rs::service::WritePropertyRequest;
            use type_conversion::{to_bacnet_object_type, to_bacnet_property_id, to_bacnet_value};

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id =
                ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);
            let bacnet_property_id = to_bacnet_property_id(property);
            let property_value_bytes = to_bacnet_value(value).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode value: {}", e))
            })?;

            let write_request = WritePropertyRequest::with_priority(
                bacnet_object_id,
                bacnet_property_id.into(),
                property_value_bytes,
                priority,
            );

            let mut service_data = Vec::new();
            write_request.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode WriteProperty: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::WriteProperty,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::WriteProperty,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    // ==========================================================================
    // Minor fix 2: WritePropertyMultiple / WritePropertyAll services
    // ==========================================================================

    /// Write a property on multiple objects atomically (confirmed service 22).
    ///
    /// Each entry contains an object identifier, property identifier, and value.
    /// If any write fails, none of the writes are applied.
    ///
    /// # Arguments
    /// * `device` - Target device ID
    /// * `entries` - List of (object, property, value) tuples
    pub fn write_property_multiple(
        &self,
        device: DeviceId,
        entries: Vec<(ObjectId, PropertyId, PropertyValue)>,
    ) -> Result<(Vec<Result<(), ProtocolError>>, Option<ObjectId>), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
            use bacnet_rs::encoding::{encode_context_object_id, encode_context_unsigned};
            use bacnet_rs::object::ObjectIdentifier;
            use type_conversion::{to_bacnet_object_type, to_bacnet_property_id};

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let mut service_data = Vec::new();

            // listOfServiceData [0] — wrap all access specs in one constructed tag
            encode_opening_tag(&mut service_data, 0).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode opening tag 0: {}", e))
            })?;

            // Encode each access specification
            for (object_id, property_id, _value) in &entries {
                // objectIdentifier [0] BACnetObjectIdentifier
                let bacnet_obj_id = ObjectIdentifier::new(
                    to_bacnet_object_type(object_id.object_type),
                    object_id.instance,
                );
                let encoded_oid = encode_context_object_id(bacnet_obj_id, 0).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode object id: {}", e))
                })?;
                service_data.extend_from_slice(&encoded_oid);

                // propertyIdentifier [1] Unsigned32
                let encoded_pid =
                    encode_context_unsigned(to_bacnet_property_id(*property_id).into(), 1)
                        .map_err(|e| {
                            ProtocolError::EncodingError(format!(
                                "Failed to encode property id: {}",
                                e
                            ))
                        })?;
                service_data.extend_from_slice(&encoded_pid);
            }

            encode_closing_tag(&mut service_data, 0).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode closing tag 0: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::WritePropertyMultiple,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            let (response_data, _service_choice) = receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::WritePropertyMultiple,
                self.request_timeout,
                &self.stats,
            )?;

            // Parse WritePropertyMultipleResponse — returns listOfServiceResult
            // Each result: objectIdentifier [0] + propertyIdentifier [1] + status [2] or errorMessage [3]
            parse_write_property_multiple_response(&response_data, &entries)
        })
    }

    /// Write a property to all priority arrays (1-16) on an object (confirmed service 23).
    ///
    /// This is a convenience wrapper around WritePropertyMultiple that writes
    /// the same value to every priority slot.
    ///
    /// # Arguments
    /// * `device` - Target device ID
    /// * `object_id` - Target object
    /// * `property_id` - Target property (must support priority arrays)
    /// * `value` - Value to write
    pub fn write_property_all(
        &self,
        device: DeviceId,
        object_id: ObjectId,
        property_id: PropertyId,
        value: PropertyValue,
    ) -> Result<Vec<Result<(), ProtocolError>>, ProtocolError> {
        // Build entries for priorities 1-16
        let entries: Vec<(ObjectId, PropertyId, PropertyValue)> = (1..=16)
            .map(|_| (object_id.clone(), property_id.clone(), value.clone()))
            .collect();

        let (results, _failed_at) = self.write_property_multiple(device, entries)?;
        Ok(results)
    }

    /// Read the object list from a device
    pub fn read_object_list(&self, device: DeviceId) -> Result<Vec<ObjectId>, ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::object::{
                ObjectIdentifier as BacnetObjectId, ObjectType as BacnetObjectType,
            };
            use bacnet_rs::service::{
                PropertyReference, ReadAccessSpecification, ReadPropertyMultipleRequest,
            };

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let device_object = BacnetObjectId::new(BacnetObjectType::Device, device);
            let property_ref = PropertyReference::new(PropertyIdentifier::ObjectList);
            let read_spec = ReadAccessSpecification::new(device_object, vec![property_ref]);
            let rpm_request = ReadPropertyMultipleRequest::new(vec![read_spec]);

            // Encode using bacnet-rs' RPM encoder
            let mut service_data = Vec::new();
            rpm_request.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode RPM: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::ReadPropertyMultiple,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            let (response_data, _) = receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::ReadPropertyMultiple,
                self.request_timeout,
                &self.stats,
            )?;

            use bacnet_rs::service::ReadPropertyMultipleResponse;
            let rpm_response =
                ReadPropertyMultipleResponse::decode(&response_data).map_err(|e| {
                    ProtocolError::DecodingError(format!(
                        "Failed to parse ReadPropertyMultiple response: {}",
                        e
                    ))
                })?;

            let mut objects = Vec::new();
            for result in rpm_response.read_access_results {
                if let Some(obj_type) =
                    type_conversion::from_bacnet_object_type(result.object_identifier.object_type)
                {
                    objects.push(ObjectId {
                        object_type: obj_type,
                        instance: result.object_identifier.instance,
                    });
                }
            }

            Ok(objects)
        })
    }

    /// Read multiple properties from a single object in one RPM request.
    pub fn read_property_multiple(
        &self,
        device: DeviceId,
        object: ObjectId,
        properties: &[PropertyId],
    ) -> Result<Vec<(PropertyId, Result<PropertyValue, ProtocolError>)>, ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::object::ObjectIdentifier as BacnetObjectId;
            use bacnet_rs::service::{
                PropertyReference, ReadAccessSpecification, ReadPropertyMultipleRequest,
                ReadPropertyMultipleResponse,
            };

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_obj_id = BacnetObjectId::new(
                type_conversion::to_bacnet_object_type(object.object_type),
                object.instance,
            );

            let prop_refs: Vec<PropertyReference> = properties
                .iter()
                .map(|&p| PropertyReference::new(type_conversion::to_bacnet_property_id(p)))
                .collect();

            let read_spec = ReadAccessSpecification::new(bacnet_obj_id, prop_refs);
            let rpm_request = ReadPropertyMultipleRequest::new(vec![read_spec]);

            let mut service_data = Vec::new();
            rpm_request.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode RPM: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::ReadPropertyMultiple,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            let (response_data, _) = receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::ReadPropertyMultiple,
                self.request_timeout,
                &self.stats,
            )?;

            let rpm_response =
                ReadPropertyMultipleResponse::decode(&response_data).map_err(|e| {
                    ProtocolError::DecodingError(format!(
                        "Failed to parse ReadPropertyMultiple response: {}",
                        e
                    ))
                })?;

            let mut results: Vec<(PropertyId, Result<PropertyValue, ProtocolError>)> = Vec::new();
            for access_result in rpm_response.read_access_results {
                for prop_result in access_result.results {
                    let our_prop_id = match prop_result.property_identifier {
                        PropertyIdentifier::PresentValue => PropertyId::PresentValue,
                        PropertyIdentifier::ObjectName => PropertyId::ObjectName,
                        PropertyIdentifier::Description => PropertyId::Description,
                        PropertyIdentifier::Units => PropertyId::Units,
                        PropertyIdentifier::StatusFlags => PropertyId::StatusFlags,
                        PropertyIdentifier::OutOfService => PropertyId::OutOfService,
                        PropertyIdentifier::Reliability => PropertyId::Reliability,
                        PropertyIdentifier::EventState => PropertyId::EventState,
                        PropertyIdentifier::Priority => PropertyId::Priority,
                        PropertyIdentifier::VendorName => PropertyId::VendorName,
                        PropertyIdentifier::ModelName => PropertyId::ModelName,
                        PropertyIdentifier::FirmwareRevision => PropertyId::FirmwareRevision,
                        PropertyIdentifier::ApplicationSoftwareVersion => {
                            PropertyId::AppSoftwareRevision
                        }
                        PropertyIdentifier::ProtocolVersion => PropertyId::ProtocolVersion,
                        PropertyIdentifier::ProtocolRevision => PropertyId::ProtocolRevision,
                        PropertyIdentifier::Location => PropertyId::Location,
                        PropertyIdentifier::ProfileName => PropertyId::ProfileName,
                        PropertyIdentifier::ObjectList => PropertyId::ObjectList,
                        PropertyIdentifier::PropertyList => PropertyId::PropertyList,
                        PropertyIdentifier::MaxApduLengthAccepted => {
                            PropertyId::MaxApduLengthAccepted
                        }
                        PropertyIdentifier::SegmentationSupported => {
                            PropertyId::SegmentationSupported
                        }
                        PropertyIdentifier::DeviceAddressBinding => {
                            PropertyId::DeviceAddressBinding
                        }
                        PropertyIdentifier::DeviceType => PropertyId::DeviceType,
                        PropertyIdentifier::MaxSegmentsAccepted => PropertyId::MaxSegmentsAccepted,
                        PropertyIdentifier::MaxInfoFrames => PropertyId::MaxInfoFrames,
                        PropertyIdentifier::ObjectType => PropertyId::ObjectType,
                        PropertyIdentifier::ListOfObjectPropertyReferences => {
                            PropertyId::ListOfObjectProperty
                        }
                        PropertyIdentifier::ApduSegmentTimeout => PropertyId::ApduSegmentTimeout,
                        PropertyIdentifier::ApduTimeout => PropertyId::ApduTimeout,
                        PropertyIdentifier::ApduLength => PropertyId::ApduLength,
                        PropertyIdentifier::LocalDate => PropertyId::LocalDate,
                        PropertyIdentifier::LocalTime => PropertyId::LocalTime,
                        PropertyIdentifier::DaylightSavingsStatus => {
                            PropertyId::DaylightSavingsStatus
                        }
                        PropertyIdentifier::TimeSynchronizationRecipients => {
                            PropertyId::TimeSynchronizationRecipients
                        }
                        PropertyIdentifier::TimeSynchronizationInterval => {
                            PropertyId::TimeSynchronizationInterval
                        }
                        PropertyIdentifier::BackupAndRestoreState => {
                            PropertyId::BackupAndRestoreState
                        }
                        PropertyIdentifier::BackupPreparationTime => {
                            PropertyId::BackupPreparationTime
                        }
                        PropertyIdentifier::RestorePreparationTime => {
                            PropertyId::RestorePreparationTime
                        }
                        PropertyIdentifier::RestoreCompletionTime => {
                            PropertyId::RestoreCompletionTime
                        }
                        PropertyIdentifier::LastRestoreTime => PropertyId::LastRestoreTime,
                        PropertyIdentifier::ConfigurationFiles => PropertyId::ConfigurationFiles,
                        PropertyIdentifier::DatabaseRevision => PropertyId::DatabaseRevision,
                        PropertyIdentifier::ActiveCovSubscriptions => {
                            PropertyId::ActiveCovSubscriptions
                        }
                        PropertyIdentifier::ActiveCovMultipleSubscriptions => {
                            PropertyId::ActiveCovMultipleSubscriptions
                        }
                        PropertyIdentifier::AckedTransitions => PropertyId::AckedTransitions,
                        PropertyIdentifier::CovIncrement => PropertyId::CovIncrement,
                        PropertyIdentifier::TimeDelay => PropertyId::TimeDelay,
                        PropertyIdentifier::NotificationClass => PropertyId::NotificationClass,
                        PropertyIdentifier::EventEnable => PropertyId::EventEnable,
                        PropertyIdentifier::EventDetectionEnable => {
                            PropertyId::EventDetectionEnable
                        }
                        PropertyIdentifier::EventAlgorithmInhibit => {
                            PropertyId::EventAlgorithmInhibit
                        }
                        PropertyIdentifier::EventAlgorithmInhibitRef => {
                            PropertyId::EventAlgorithmInhibitRef
                        }
                        PropertyIdentifier::NotifyType => PropertyId::NotifyType,
                        PropertyIdentifier::EventTimeStamps => PropertyId::EventTimeStamps,
                        PropertyIdentifier::EventMessageTexts => PropertyId::EventMessageTexts,
                        PropertyIdentifier::EventMessageTextsConfig => {
                            PropertyId::EventMessageTextsConfig
                        }
                        PropertyIdentifier::PriorityForWriting => PropertyId::PriorityForWriting,
                        PropertyIdentifier::AlarmValue => PropertyId::AlarmValue,
                        PropertyIdentifier::AlarmValues => PropertyId::AlarmValues,
                        PropertyIdentifier::FaultValues => PropertyId::FaultValues,
                        PropertyIdentifier::Setpoint => PropertyId::Setpoint,
                        PropertyIdentifier::SetpointReference => PropertyId::SetpointReference,
                        PropertyIdentifier::LogDeviceObjectProperty => {
                            PropertyId::LogDeviceObjectProperty
                        }
                        PropertyIdentifier::LoggingType => PropertyId::LoggingType,
                        PropertyIdentifier::LogInterval => PropertyId::LogInterval,
                        PropertyIdentifier::LoggingObject => PropertyId::LogObject,
                        PropertyIdentifier::LoggingRecord => PropertyId::LoggingRecord,
                        PropertyIdentifier::RecordsSinceNotification => {
                            PropertyId::RecordsSinceNotification
                        }
                        PropertyIdentifier::LastNotifyRecord => PropertyId::LastNotifyRecord,
                        PropertyIdentifier::NotificationThreshold => {
                            PropertyId::NotificationThreshold
                        }
                        PropertyIdentifier::BufferSize => PropertyId::BufferSize,
                        PropertyIdentifier::RecordCount => PropertyId::RecordCount,
                        PropertyIdentifier::TotalRecordCount => PropertyId::TotalRecordCount,
                        PropertyIdentifier::StartTime => PropertyId::StartTime,
                        PropertyIdentifier::StopTime => PropertyId::StopTime,
                        PropertyIdentifier::LogBuffer => PropertyId::LogBuffer,
                        PropertyIdentifier::Enable => PropertyId::Enable,
                        PropertyIdentifier::NetworkNumber => PropertyId::NetworkNumber,
                        PropertyIdentifier::NetworkNumberQuality => {
                            PropertyId::NetworkNumberQuality
                        }
                        PropertyIdentifier::NetworkType => PropertyId::NetworkType,
                        PropertyIdentifier::RoutingTable => PropertyId::RoutingTable,
                        PropertyIdentifier::LinkSpeed => PropertyId::LinkSpeed,
                        PropertyIdentifier::LinkSpeeds => PropertyId::LinkSpeeds,
                        PropertyIdentifier::LinkSpeedAutonegotiate => {
                            PropertyId::LinkSpeedAutonegotiate
                        }
                        PropertyIdentifier::StructuredObjectList => {
                            PropertyId::StructuredObjectList
                        }
                        PropertyIdentifier::SubordinateList => PropertyId::SubordinateList,
                        PropertyIdentifier::SubordinateNodeTypes => {
                            PropertyId::SubordinateNodeTypes
                        }
                        PropertyIdentifier::SubordinateAnnotations => {
                            PropertyId::SubordinateAnnotations
                        }
                        PropertyIdentifier::SubordinateRelationships => {
                            PropertyId::SubordinateRelationships
                        }
                        PropertyIdentifier::SubordinateTags => PropertyId::SubordinateTags,
                        PropertyIdentifier::ProfileLocation => PropertyId::ProfileLocation,
                        PropertyIdentifier::ValueSource => PropertyId::ValueSource,
                        PropertyIdentifier::ValueSourceArray => PropertyId::ValueSourceArray,
                        PropertyIdentifier::CommandTimeArray => PropertyId::CommandTimeArray,
                        _ => continue,
                    };

                    let value = match prop_result.value {
                        bacnet_rs::service::PropertyResultValue::Value(ref vals) => {
                            if let Some(val) = vals.first() {
                                match type_conversion::convert_bacnet_property_value(val) {
                                    Ok(v) => Ok(v),
                                    Err(e) => Err(ProtocolError::DecodingError(e)),
                                }
                            } else {
                                Err(ProtocolError::DecodingError(
                                    "Empty property value list".to_string(),
                                ))
                            }
                        }
                        bacnet_rs::service::PropertyResultValue::Error(class, code) => {
                            Err(ProtocolError::BacnetError {
                                class: map_error_class(class as u8),
                                code: map_error_code(code as u8),
                            })
                        }
                    };

                    results.push((our_prop_id, value));
                }
            }

            Ok(results)
        })
    }
}

// ── COV (Change of Value) support ─────────────────────────────────────────────

/// A decoded COV notification
#[derive(Debug, Clone)]
pub struct CovNotification {
    /// Device instance that sent the notification
    pub device_id: DeviceId,
    /// Source address (for routing replies)
    pub source_address: Address,
    /// Subscriber process ID
    pub subscriber_process_id: u32,
    /// The object that changed
    pub object_id: ObjectId,
    /// Remaining subscription lifetime in seconds (optional)
    pub time_remaining: Option<u32>,
    /// List of (property_identifier, property_value) that changed
    pub changed_values: Vec<(PropertyId, PropertyValue)>,
}

/// Decode an unconfirmed COV notification from raw payload bytes.
pub fn decode_cov_notification(
    payload: &[u8],
    source_address: Address,
) -> Result<CovNotification, ProtocolError> {
    use crate::type_conversion::from_bacnet_object_type;
    use bacnet_rs::encoding::{
        decode_context_object_id, decode_context_tag, decode_context_unsigned, decode_tag,
        BACnetTag,
    };

    let mut pos = 0;

    // subscriberProcessIdentifier [0] Unsigned32
    let (subscriber_process_id, consumed) =
        decode_context_unsigned(&payload[pos..], 0).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode subscriber process id: {}", e))
        })?;
    pos += consumed;

    // initiatingDeviceIdentifier [1] BACnetObjectIdentifier
    let (device_obj_id, consumed) = decode_context_object_id(&payload[pos..], 1).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode initiating device id: {}", e))
    })?;
    pos += consumed;
    let device_id = device_obj_id.instance;

    // monitoredObjectIdentifier [2] BACnetObjectIdentifier
    let (monitored_obj_id, consumed) =
        decode_context_object_id(&payload[pos..], 2).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode monitored object id: {}", e))
        })?;
    pos += consumed;
    let object_type = from_bacnet_object_type(monitored_obj_id.object_type).ok_or_else(|| {
        ProtocolError::DecodingError(format!(
            "Unknown BACnet object type: {:?}",
            monitored_obj_id.object_type
        ))
    })?;
    let object_id = ObjectId {
        object_type,
        instance: monitored_obj_id.instance,
    };

    // timeRemaining [3] Unsigned32 OPTIONAL — check if next is tag 3
    let time_remaining = if pos < payload.len() {
        let tag_byte = payload[pos];
        let is_context = (tag_byte & 0x08) != 0;
        let tag_number = (tag_byte >> 4) & 0x0F;
        if is_context && tag_number == 3 {
            let (val, consumed) = decode_context_unsigned(&payload[pos..], 3).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode time remaining: {}", e))
            })?;
            pos += consumed;
            Some(val)
        } else {
            None
        }
    } else {
        None
    };

    // listOfValues [4] SEQUENCE OF BACnetPropertyValue
    if pos >= payload.len() {
        return Err(ProtocolError::DecodingError(
            "Missing listOfValues opening tag".to_string(),
        ));
    }

    // Check for opening tag 4
    let (tag, _tag_len, tag_consumed) = decode_tag(&payload[pos..]).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode listOfValues tag: {}", e))
    })?;
    if tag != BACnetTag::Context(4) {
        return Err(ProtocolError::DecodingError(format!(
            "Expected opening tag 4 for listOfValues, got {:?}",
            tag
        )));
    }
    // Check if it's an opening tag (low nibble 0x0E) or context tag with data
    let is_opening = (payload[pos] & 0x0F) == 0x0E;
    if is_opening {
        pos += tag_consumed; // skip opening tag 4
    } else {
        return Err(ProtocolError::DecodingError(
            "Expected opening tag 4 (0x4E)".to_string(),
        ));
    }

    // Decode property values until we hit closing tag 4
    let mut changed_values: Vec<(PropertyId, PropertyValue)> = Vec::new();

    while pos < payload.len() {
        let tag_byte = payload[pos];
        let is_closing = (tag_byte & 0x0F) == 0x0F;
        let tag_number = (tag_byte >> 4) & 0x0F;
        if is_closing && tag_number == 4 {
            break;
        }

        let value_pos_before = pos;

        // Parse propertyIdentifier [0]
        let (prop_enum, consumed) = decode_context_unsigned(&payload[pos..], 0).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode property id: {}", e))
        })?;
        pos += consumed;

        // Parse propertyArrayIndex [1] OPTIONAL (skip if present)
        if pos < payload.len() {
            let byte = payload[pos];
            let is_ctx = (byte & 0x08) != 0;
            let tn = (byte >> 4) & 0x0F;
            if is_ctx && tn == 1 {
                let (_, consumed) = decode_context_unsigned(&payload[pos..], 1).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode array index: {}", e))
                })?;
                pos += consumed;
            }
        }

        // Parse propertyValue [2] — expect context tag 2 (opening or primitive)
        if pos >= payload.len() {
            return Err(ProtocolError::DecodingError(
                "Unexpected end of property value".to_string(),
            ));
        }

        let (val_tag, _val_len, val_consumed) = decode_tag(&payload[pos..]).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode property value tag: {}", e))
        })?;
        if val_tag != BACnetTag::Context(2) {
            return Err(ProtocolError::DecodingError(format!(
                "Expected tag 2 for property value, got {:?}",
                val_tag
            )));
        }

        let val_is_opening = (payload[pos] & 0x0F) == 0x0E;
        if val_is_opening {
            pos += val_consumed; // skip opening tag 2
                                 // Find closing tag 2 and extract bytes in between
            let value_start = pos;
            while pos < payload.len() {
                let byte = payload[pos];
                let is_close = (byte & 0x0F) == 0x0F;
                let tn = (byte >> 4) & 0x0F;
                if is_close && tn == 2 {
                    let value_bytes = &payload[value_start..pos];
                    if !value_bytes.is_empty() {
                        match crate::type_conversion::from_bacnet_value(value_bytes) {
                            Ok(pv) => {
                                changed_values.push((map_unsigned_to_property_id(prop_enum), pv));
                            }
                            Err(e) => {
                                // Log and skip unparseable values
                                eprintln!("Failed to decode COV property value: {}", e);
                            }
                        }
                    }
                    pos += 1; // skip closing tag
                    break;
                }
                pos += 1;
            }
        } else {
            // Primitive context-tagged value
            let (_, len, consumed) = decode_context_tag(&payload[pos..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode value context tag: {}", e))
            })?;
            pos += consumed;
            if len > 0 && pos + len <= payload.len() {
                let value_bytes = &payload[pos..pos + len];
                pos += len;
                match crate::type_conversion::from_bacnet_value(value_bytes) {
                    Ok(pv) => {
                        changed_values.push((map_unsigned_to_property_id(prop_enum), pv));
                    }
                    Err(e) => {
                        eprintln!("Failed to decode COV property value: {}", e);
                    }
                }
            } else {
                // Zero-length value — skip
            }
        }

        // Parse priority [3] OPTIONAL (skip if present)
        if pos < payload.len() {
            let byte = payload[pos];
            let is_ctx = (byte & 0x08) != 0;
            let tn = (byte >> 4) & 0x0F;
            if is_ctx && tn == 3 {
                let (_, consumed) = decode_context_unsigned(&payload[pos..], 3).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode priority: {}", e))
                })?;
                pos += consumed;
            }
        }

        // If we didn't advance, skip one byte to avoid infinite loop
        if pos == value_pos_before {
            pos += 1;
        }
    }

    Ok(CovNotification {
        device_id,
        source_address,
        subscriber_process_id,
        object_id,
        time_remaining,
        changed_values,
    })
}

/// Decode an unconfirmed event notification from raw payload bytes.
fn parse_event_notification_response(
    payload: &[u8],
) -> Result<EventNotificationInfo, ProtocolError> {
    use crate::type_conversion::from_bacnet_object_type;
    use bacnet_rs::encoding::{
        decode_context_object_id, decode_context_tag, decode_context_unsigned, decode_tag,
        BACnetTag,
    };

    let mut pos = 0;

    // initiatingDeviceIdentifier [0] BACnetObjectIdentifier
    let (device_obj_id, consumed) = decode_context_object_id(&payload[pos..], 0).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode initiating device: {}", e))
    })?;
    pos += consumed;
    let device_id = device_obj_id.instance;

    // eventObjectIdentifier [1] BACnetObjectIdentifier
    let (event_obj_id, consumed) = decode_context_object_id(&payload[pos..], 1).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode event object: {}", e))
    })?;
    pos += consumed;
    let object_type = from_bacnet_object_type(event_obj_id.object_type).ok_or_else(|| {
        ProtocolError::DecodingError(format!(
            "Unknown BACnet object type: {:?}",
            event_obj_id.object_type
        ))
    })?;
    let event_object = ObjectId {
        object_type,
        instance: event_obj_id.instance,
    };

    // timestamp [2] UTCTime OPTIONAL
    let mut timestamp: Option<String> = None;
    if pos < payload.len() {
        let (tag, _tag_len, tag_consumed) = decode_tag(&payload[pos..])
            .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode tag: {}", e)))?;
        if tag == BACnetTag::Context(2) {
            pos += tag_consumed;
            if (payload[pos] & 0x0F) == 0x0E {
                // Opening tag — find closing tag 2
                pos += 1;
                let start = pos;
                while pos < payload.len()
                    && !(payload[pos] == 0x12 && (payload[pos] & 0x0F == 0x02))
                {
                    pos += 1;
                }
                timestamp = Some(String::from_utf8_lossy(&payload[start..pos]).to_string());
            } else {
                let (_, len, consumed) = decode_context_tag(&payload[pos..]).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode timestamp: {}", e))
                })?;
                pos += consumed;
                if len > 0 && pos + len <= payload.len() {
                    timestamp = Some(String::from_utf8_lossy(&payload[pos..pos + len]).to_string());
                    pos += len;
                }
            }
        }
    }

    // notificationClass [3] Unsigned32
    let (notification_class, consumed) =
        decode_context_unsigned(&payload[pos..], 3).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode notification class: {}", e))
        })?;
    pos += consumed;

    // priority [4] Unsigned32
    let (priority, consumed) = decode_context_unsigned(&payload[pos..], 4)
        .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode priority: {}", e)))?;
    pos += consumed;

    // eventType [5] ENUMERATED
    let (event_type, consumed) = decode_context_unsigned(&payload[pos..], 5)
        .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode event type: {}", e)))?;
    pos += consumed;

    // notifyType [6] ENUMERATED
    let (notify_type, consumed) = decode_context_unsigned(&payload[pos..], 6).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode notify type: {}", e))
    })?;
    pos += consumed;

    // ackRequired [7] BOOLEAN
    let (ack_required, consumed) = decode_context_unsigned(&payload[pos..], 7).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode ack required: {}", e))
    })?;
    pos += consumed;

    // eventState [8] ENUMERATED
    let (event_state, consumed) = decode_context_unsigned(&payload[pos..], 8).map_err(|e| {
        ProtocolError::DecodingError(format!("Failed to decode event state: {}", e))
    })?;
    pos += consumed;

    Ok(EventNotificationInfo {
        initiating_device: device_id,
        event_object,
        timestamp,
        notification_class,
        priority,
        event_type,
        notify_type,
        ack_required: ack_required != 0,
        event_state,
    })
}

/// Parse ConfirmedEventNotificationAck response data.
fn parse_confirmed_event_notification_ack(
    data: &[u8],
) -> Result<Vec<EventNotificationResponse>, ProtocolError> {
    use crate::type_conversion::from_bacnet_object_type;
    use bacnet_rs::encoding::{
        decode_context_object_id, decode_context_tag, decode_context_unsigned, decode_tag,
        BACnetTag,
    };

    let mut responses = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Each response is wrapped in context tag 0 (opening tag)
        let (tag, _tag_len, tag_consumed) = decode_tag(&data[pos..])
            .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode tag: {}", e)))?;
        if tag != BACnetTag::Context(0) {
            break;
        }
        pos += tag_consumed;

        let inner_start = pos;
        if (data[pos] & 0x0F) == 0x0E {
            // Opening tag — find closing tag 0
            pos += 1;
            while pos < data.len() && !(data[pos] == 0x10 && (data[pos] & 0x0F == 0x00)) {
                pos += 1;
            }
            // Skip closing tag 0
            if pos < data.len() {
                pos += 1;
            }
        } else {
            let (_, len, consumed) = decode_context_tag(&data[pos..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode tag len: {}", e))
            })?;
            pos += consumed;
            pos += len;
        }

        let inner_data = &data[inner_start..pos.min(data.len())];
        let mut i = 0;

        // eventObjectIdentifier [0]
        let (obj_id, consumed) = decode_context_object_id(&inner_data[i..], 0).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode event object in ack: {}", e))
        })?;
        i += consumed;
        let object_type = from_bacnet_object_type(obj_id.object_type).ok_or_else(|| {
            ProtocolError::DecodingError(format!(
                "Unknown object type in ack: {:?}",
                obj_id.object_type
            ))
        })?;
        let event_object = ObjectId {
            object_type,
            instance: obj_id.instance,
        };

        let mut event_state = 0u32;
        let mut event_type = 0u32;
        let mut notify_type = 0u32;
        let mut event_enable = Vec::new();
        let mut event_priorities = Vec::new();
        let mut ack_required = false;
        let mut event_time_stamps = Vec::new();
        let mut event_message_text: Option<String> = None;
        let mut optional_context: Option<String> = None;
        let mut local_timestamp: Option<String> = None;

        while i < inner_data.len() {
            let (tag, _tag_len, tag_consumed) = decode_tag(&inner_data[i..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode inner tag: {}", e))
            })?;

            match tag {
                BACnetTag::Context(1) => {
                    // eventState
                    let (val, _) = decode_context_unsigned(&inner_data[i..], 1).map_err(|e| {
                        ProtocolError::DecodingError(format!("Failed to decode eventState: {}", e))
                    })?;
                    event_state = val;
                }
                BACnetTag::Context(2) => {
                    // eventType
                    let (val, _) = decode_context_unsigned(&inner_data[i..], 2).map_err(|e| {
                        ProtocolError::DecodingError(format!("Failed to decode event type: {}", e))
                    })?;
                    event_type = val;
                }
                BACnetTag::Context(3) => {
                    // notifyType
                    let (val, _) = decode_context_unsigned(&inner_data[i..], 3).map_err(|e| {
                        ProtocolError::DecodingError(format!("Failed to decode notify type: {}", e))
                    })?;
                    notify_type = val;
                }
                BACnetTag::Context(4) => {
                    // eventEnable — bitstring
                    let (_, len, _) = decode_context_tag(&inner_data[i..]).map_err(|e| {
                        ProtocolError::DecodingError(format!("Failed to decode eventEnable: {}", e))
                    })?;
                    if len > 0 {
                        let start = i + tag_consumed;
                        event_enable = inner_data[start..start + len]
                            .iter()
                            .map(|b| b & 1 != 0)
                            .collect();
                    }
                }
                BACnetTag::Context(5) => {
                    // eventPriorities
                    let (_, len, _) = decode_context_tag(&inner_data[i..]).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to decode eventPriorities: {}",
                            e
                        ))
                    })?;
                    if len > 0 {
                        let start = i + tag_consumed;
                        let bits = &inner_data[start..start + len];
                        event_priorities = bits.iter().map(|b| *b as u32).collect();
                    }
                }
                BACnetTag::Context(6) => {
                    // ackRequired
                    let (val, _) = decode_context_unsigned(&inner_data[i..], 6).map_err(|e| {
                        ProtocolError::DecodingError(format!("Failed to decode ackRequired: {}", e))
                    })?;
                    ack_required = val != 0;
                }
                BACnetTag::Context(7) => {
                    // eventTimeStamps — bitstring
                    let (_, len, _) = decode_context_tag(&inner_data[i..]).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to decode eventTimeStamps: {}",
                            e
                        ))
                    })?;
                    if len > 0 {
                        let start = i + tag_consumed;
                        let bits = &inner_data[start..start + len];
                        event_time_stamps =
                            bits.iter().map(|b| Some(format!("{:08b}", b))).collect();
                    }
                }
                BACnetTag::Context(8) => {
                    // eventMessageText — character string
                    let (_, len, _) = decode_context_tag(&inner_data[i..]).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to decode eventMessageText: {}",
                            e
                        ))
                    })?;
                    if len > 0 {
                        let start = i + tag_consumed;
                        event_message_text = Some(
                            String::from_utf8_lossy(&inner_data[start..start + len]).to_string(),
                        );
                    }
                }
                BACnetTag::Context(9) => {
                    // optionalContext
                    let (_, len, _) = decode_context_tag(&inner_data[i..]).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to decode optionalContext: {}",
                            e
                        ))
                    })?;
                    if len > 0 {
                        let start = i + tag_consumed;
                        optional_context = Some(
                            String::from_utf8_lossy(&inner_data[start..start + len]).to_string(),
                        );
                    }
                }
                BACnetTag::Context(10) => {
                    // localTimestamp
                    let (_, len, _) = decode_context_tag(&inner_data[i..]).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to decode localTimestamp: {}",
                            e
                        ))
                    })?;
                    if len > 0 {
                        let start = i + tag_consumed;
                        local_timestamp = Some(
                            String::from_utf8_lossy(&inner_data[start..start + len]).to_string(),
                        );
                    }
                }
                _ => break,
            }
            i += tag_consumed;
        }

        responses.push(EventNotificationResponse {
            event_object,
            event_state,
            event_type,
            notify_type,
            event_enable,
            event_priorities,
            ack_required,
            event_time_stamps,
            event_message_text,
            optional_context,
            local_timestamp,
        });
    }

    Ok(responses)
}

/// Parse AcknowledgeAlarm response data.
fn parse_acknowledge_alarm_response(
    data: &[u8],
) -> Result<AcknowledgeAlarmResponse, ProtocolError> {
    use bacnet_rs::encoding::{decode_context_tag, decode_context_unsigned, decode_tag, BACnetTag};

    let mut acknowledged_state_changed = false;
    let mut acked_transitions = Vec::new();
    let mut acked_transitions_time = Vec::new();

    let mut pos = 0;
    while pos < data.len() {
        let (tag, _tag_len, tag_consumed) = decode_tag(&data[pos..]).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode ack tag: {}", e))
        })?;

        match tag {
            BACnetTag::Context(0) => {
                // acknowledgedState
                let (val, _) = decode_context_unsigned(&data[pos..], 0).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode ack state: {}", e))
                })?;
                acknowledged_state_changed = val != 0;
            }
            BACnetTag::Context(1) => {
                // ackedTransitions — bitstring
                let (_, len, _) = decode_context_tag(&data[pos..]).map_err(|e| {
                    ProtocolError::DecodingError(format!(
                        "Failed to decode acked transitions: {}",
                        e
                    ))
                })?;
                if len > 0 {
                    let start = pos + tag_consumed;
                    acked_transitions = data[start..start + len]
                        .iter()
                        .map(|b| b & 1 != 0)
                        .collect();
                }
            }
            BACnetTag::Context(2) => {
                // ackedTransitionsTime — UTCTime
                let (_, len, _) = decode_context_tag(&data[pos..]).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode acked time: {}", e))
                })?;
                if len > 0 {
                    let start = pos + tag_consumed;
                    acked_transitions_time.push(Some(
                        String::from_utf8_lossy(&data[start..start + len]).to_string(),
                    ));
                }
            }
            _ => break,
        }
        pos += tag_consumed;
    }

    Ok(AcknowledgeAlarmResponse {
        acknowledged_state_changed,
        acked_transitions,
        acked_transitions_time,
    })
}

/// Parse WritePropertyMultiple response.
fn parse_write_property_multiple_response(
    data: &[u8],
    entries: &[(ObjectId, PropertyId, PropertyValue)],
) -> Result<(Vec<Result<(), ProtocolError>>, Option<ObjectId>), ProtocolError> {
    use bacnet_rs::encoding::{decode_context_tag, decode_tag, BACnetTag};

    let mut results = Vec::new();
    let mut failed_at: Option<ObjectId> = None;

    let mut pos = 0;
    while pos < data.len() && results.len() < entries.len() {
        let (tag, _tag_len, tag_consumed) = decode_tag(&data[pos..]).map_err(|e| {
            ProtocolError::DecodingError(format!("Failed to decode result tag: {}", e))
        })?;

        if tag == BACnetTag::Context(2) {
            let (_, len, _) = decode_context_tag(&data[pos..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode status: {}", e))
            })?;
            pos += tag_consumed;
            if len > 0 {
                pos += len;
            }
            if results.len() < entries.len() {
                results.push(Ok(()));
            }
        } else if tag == BACnetTag::Context(3) {
            let (_, len, _) = decode_context_tag(&data[pos..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode error: {}", e))
            })?;
            pos += tag_consumed;
            let error_msg = if len > 0 {
                let start = pos;
                pos += len;
                String::from_utf8_lossy(&data[start..start + len]).to_string()
            } else {
                "Unknown error".to_string()
            };
            if let Some((ref obj, _, _)) = entries.get(results.len()) {
                failed_at = Some(obj.clone());
            }
            if results.len() < entries.len() {
                results.push(Err(ProtocolError::DecodingError(error_msg)));
            }
        } else {
            pos += tag_consumed;
        }
    }

    Ok((results, failed_at))
}

fn map_unsigned_to_property_id(val: u32) -> PropertyId {
    use bacnet_rs::object::property_identifier::PropertyIdentifier;
    // Try to interpret as a BACnet property identifier then map
    // Use bacnet-rs PropertyIdentifier::from and match
    let pi = PropertyIdentifier::from(val);
    match pi {
        // Core types
        PropertyIdentifier::PresentValue => PropertyId::PresentValue,
        PropertyIdentifier::ObjectName => PropertyId::ObjectName,
        PropertyIdentifier::Description => PropertyId::Description,
        PropertyIdentifier::Units => PropertyId::Units,
        PropertyIdentifier::StatusFlags => PropertyId::StatusFlags,
        PropertyIdentifier::OutOfService => PropertyId::OutOfService,
        PropertyIdentifier::Reliability => PropertyId::Reliability,
        PropertyIdentifier::EventState => PropertyId::EventState,
        PropertyIdentifier::Priority => PropertyId::Priority,
        // Device/Vendor
        PropertyIdentifier::VendorName => PropertyId::VendorName,
        PropertyIdentifier::ModelName => PropertyId::ModelName,
        PropertyIdentifier::FirmwareRevision => PropertyId::FirmwareRevision,
        PropertyIdentifier::ApplicationSoftwareVersion => PropertyId::AppSoftwareRevision,
        PropertyIdentifier::ProtocolVersion => PropertyId::ProtocolVersion,
        PropertyIdentifier::ProtocolRevision => PropertyId::ProtocolRevision,
        PropertyIdentifier::Location => PropertyId::Location,
        PropertyIdentifier::ProfileName => PropertyId::ProfileName,
        // Lists/Capabilities
        PropertyIdentifier::ObjectList => PropertyId::ObjectList,
        PropertyIdentifier::PropertyList => PropertyId::PropertyList,
        PropertyIdentifier::MaxApduLengthAccepted => PropertyId::MaxApduLengthAccepted,
        PropertyIdentifier::SegmentationSupported => PropertyId::SegmentationSupported,
        PropertyIdentifier::DeviceAddressBinding => PropertyId::DeviceAddressBinding,
        PropertyIdentifier::DeviceType => PropertyId::DeviceType,
        PropertyIdentifier::MaxSegmentsAccepted => PropertyId::MaxSegmentsAccepted,
        PropertyIdentifier::MaxInfoFrames => PropertyId::MaxInfoFrames,
        PropertyIdentifier::ObjectType => PropertyId::ObjectType,
        PropertyIdentifier::ListOfObjectPropertyReferences => PropertyId::ListOfObjectProperty,
        // APDU/Timeout
        PropertyIdentifier::ApduSegmentTimeout => PropertyId::ApduSegmentTimeout,
        PropertyIdentifier::ApduTimeout => PropertyId::ApduTimeout,
        PropertyIdentifier::ApduLength => PropertyId::ApduLength,
        // TimeSync
        PropertyIdentifier::LocalDate => PropertyId::LocalDate,
        PropertyIdentifier::LocalTime => PropertyId::LocalTime,
        PropertyIdentifier::DaylightSavingsStatus => PropertyId::DaylightSavingsStatus,
        PropertyIdentifier::TimeSynchronizationRecipients => {
            PropertyId::TimeSynchronizationRecipients
        }
        PropertyIdentifier::TimeSynchronizationInterval => PropertyId::TimeSynchronizationInterval,
        // Backup/Restore
        PropertyIdentifier::BackupAndRestoreState => PropertyId::BackupAndRestoreState,
        PropertyIdentifier::BackupPreparationTime => PropertyId::BackupPreparationTime,
        PropertyIdentifier::RestorePreparationTime => PropertyId::RestorePreparationTime,
        PropertyIdentifier::RestoreCompletionTime => PropertyId::RestoreCompletionTime,
        PropertyIdentifier::LastRestoreTime => PropertyId::LastRestoreTime,
        PropertyIdentifier::ConfigurationFiles => PropertyId::ConfigurationFiles,
        PropertyIdentifier::DatabaseRevision => PropertyId::DatabaseRevision,
        PropertyIdentifier::ActiveCovSubscriptions => PropertyId::ActiveCovSubscriptions,
        PropertyIdentifier::ActiveCovMultipleSubscriptions => {
            PropertyId::ActiveCovMultipleSubscriptions
        }
        // Alarming/Event
        PropertyIdentifier::AckedTransitions => PropertyId::AckedTransitions,
        PropertyIdentifier::CovIncrement => PropertyId::CovIncrement,
        PropertyIdentifier::TimeDelay => PropertyId::TimeDelay,
        PropertyIdentifier::NotificationClass => PropertyId::NotificationClass,
        PropertyIdentifier::EventEnable => PropertyId::EventEnable,
        PropertyIdentifier::EventDetectionEnable => PropertyId::EventDetectionEnable,
        PropertyIdentifier::EventAlgorithmInhibit => PropertyId::EventAlgorithmInhibit,
        PropertyIdentifier::EventAlgorithmInhibitRef => PropertyId::EventAlgorithmInhibitRef,
        PropertyIdentifier::NotifyType => PropertyId::NotifyType,
        PropertyIdentifier::EventTimeStamps => PropertyId::EventTimeStamps,
        PropertyIdentifier::EventMessageTexts => PropertyId::EventMessageTexts,
        PropertyIdentifier::EventMessageTextsConfig => PropertyId::EventMessageTextsConfig,
        PropertyIdentifier::PriorityForWriting => PropertyId::PriorityForWriting,
        PropertyIdentifier::AlarmValue => PropertyId::AlarmValue,
        PropertyIdentifier::AlarmValues => PropertyId::AlarmValues,
        PropertyIdentifier::FaultValues => PropertyId::FaultValues,
        PropertyIdentifier::Setpoint => PropertyId::Setpoint,
        PropertyIdentifier::SetpointReference => PropertyId::SetpointReference,
        // Trending/Logging
        PropertyIdentifier::LogDeviceObjectProperty => PropertyId::LogDeviceObjectProperty,
        PropertyIdentifier::LoggingType => PropertyId::LoggingType,
        PropertyIdentifier::LogInterval => PropertyId::LogInterval,
        PropertyIdentifier::LoggingObject => PropertyId::LogObject,
        PropertyIdentifier::LoggingRecord => PropertyId::LoggingRecord,
        PropertyIdentifier::RecordsSinceNotification => PropertyId::RecordsSinceNotification,
        PropertyIdentifier::LastNotifyRecord => PropertyId::LastNotifyRecord,
        PropertyIdentifier::NotificationThreshold => PropertyId::NotificationThreshold,
        PropertyIdentifier::BufferSize => PropertyId::BufferSize,
        PropertyIdentifier::RecordCount => PropertyId::RecordCount,
        PropertyIdentifier::TotalRecordCount => PropertyId::TotalRecordCount,
        PropertyIdentifier::StartTime => PropertyId::StartTime,
        PropertyIdentifier::StopTime => PropertyId::StopTime,
        PropertyIdentifier::LogBuffer => PropertyId::LogBuffer,
        PropertyIdentifier::Enable => PropertyId::Enable,
        // Network
        PropertyIdentifier::NetworkNumber => PropertyId::NetworkNumber,
        PropertyIdentifier::NetworkNumberQuality => PropertyId::NetworkNumberQuality,
        PropertyIdentifier::NetworkType => PropertyId::NetworkType,
        PropertyIdentifier::RoutingTable => PropertyId::RoutingTable,
        PropertyIdentifier::LinkSpeed => PropertyId::LinkSpeed,
        PropertyIdentifier::LinkSpeeds => PropertyId::LinkSpeeds,
        PropertyIdentifier::LinkSpeedAutonegotiate => PropertyId::LinkSpeedAutonegotiate,
        // StructuredView
        PropertyIdentifier::StructuredObjectList => PropertyId::StructuredObjectList,
        PropertyIdentifier::SubordinateList => PropertyId::SubordinateList,
        PropertyIdentifier::SubordinateNodeTypes => PropertyId::SubordinateNodeTypes,
        PropertyIdentifier::SubordinateAnnotations => PropertyId::SubordinateAnnotations,
        PropertyIdentifier::SubordinateRelationships => PropertyId::SubordinateRelationships,
        PropertyIdentifier::SubordinateTags => PropertyId::SubordinateTags,
        // Other
        PropertyIdentifier::ProfileLocation => PropertyId::ProfileLocation,
        PropertyIdentifier::ValueSource => PropertyId::ValueSource,
        PropertyIdentifier::ValueSourceArray => PropertyId::ValueSourceArray,
        PropertyIdentifier::CommandTimeArray => PropertyId::CommandTimeArray,
        _ => {
            // For unmapped properties, return PresentValue as default
            match val {
                85 => PropertyId::ObjectList,
                600 => PropertyId::NetworkAccessSecurity,
                601 => PropertyId::NetworkPriority,
                602 => PropertyId::RouterEntryDiscoveryTime,
                603 => PropertyId::PortLevel,
                604 => PropertyId::PortNumber,
                605 => PropertyId::ConstantValue,
                606 => PropertyId::DescriptionOfSchedule,
                607 => PropertyId::EventAlarmInhibited,
                608 => PropertyId::NotificationThresholdCount,
                _ => PropertyId::PresentValue,
            }
        }
    }
}

impl BacnetService {
    /// Subscribe for COV notifications on an object.
    ///
    /// When `lifetime` is None, subscription is permanent until cancelled.
    /// When `issue_confirmed` is true, the device sends ConfirmedCOVNotification (needs ack).
    pub fn subscribe_cov(
        &self,
        device: DeviceId,
        object: ObjectId,
        subscriber_process_id: u32,
        lifetime: Option<u32>,
        issue_confirmed: bool,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::service::SubscribeCovRequest;
            use type_conversion::to_bacnet_object_type;

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id =
                ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);

            let sub_req = if let Some(lifetime_secs) = lifetime {
                let req = SubscribeCovRequest::with_lifetime(
                    subscriber_process_id,
                    bacnet_object_id,
                    lifetime_secs,
                );
                if issue_confirmed {
                    SubscribeCovRequest {
                        issue_confirmed_notifications: Some(true),
                        ..req
                    }
                } else {
                    req
                }
            } else {
                SubscribeCovRequest::with_confirmation(
                    subscriber_process_id,
                    bacnet_object_id,
                    issue_confirmed,
                )
            };

            let mut service_data = Vec::new();
            sub_req.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode SubscribeCOV: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::SubscribeCOV,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::SubscribeCOV,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    /// Subscribe for COV notifications on a specific property.
    pub fn subscribe_cov_property(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        subscriber_process_id: u32,
        lifetime: Option<u32>,
        issue_confirmed: bool,
        cov_increment: Option<f32>,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
            use bacnet_rs::encoding::{
                encode_context_object_id, encode_context_unsigned, encode_real,
            };
            use bacnet_rs::service::{PropertyReference, SubscribeCovPropertyRequest};
            use type_conversion::{to_bacnet_object_type, to_bacnet_property_id};

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id =
                ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);

            let bacnet_property_id = to_bacnet_property_id(property);
            let property_ref = PropertyReference::new(bacnet_property_id);

            let mut req = SubscribeCovPropertyRequest::new(
                subscriber_process_id,
                bacnet_object_id,
                property_ref,
            );

            if issue_confirmed {
                req.issue_confirmed_notifications = Some(true);
            }
            if let Some(lt) = lifetime {
                req.lifetime = Some(lt);
            }
            if let Some(increment) = cov_increment {
                req = req.with_cov_increment(increment);
            }

            // Encode manually since SubscribeCovPropertyRequest doesn't have encode()
            let mut service_data = Vec::new();

            // subscriberProcessIdentifier [0] Unsigned32
            let encoded = encode_context_unsigned(subscriber_process_id, 0).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode process id: {}", e))
            })?;
            service_data.extend_from_slice(&encoded);

            // monitoredObjectIdentifier [1] BACnetObjectIdentifier
            let encoded_oid = encode_context_object_id(bacnet_object_id, 1).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode object id: {}", e))
            })?;
            service_data.extend_from_slice(&encoded_oid);

            // issueConfirmedNotifications [2] BOOLEAN OPTIONAL
            if let Some(confirmed) = req.issue_confirmed_notifications {
                let encoded =
                    encode_context_unsigned(if confirmed { 1 } else { 0 }, 2).map_err(|e| {
                        ProtocolError::EncodingError(format!(
                            "Failed to encode confirmed flag: {}",
                            e
                        ))
                    })?;
                service_data.extend_from_slice(&encoded);
            }

            // lifetime [3] Unsigned32 OPTIONAL
            if let Some(lt) = req.lifetime {
                let encoded = encode_context_unsigned(lt, 3).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode lifetime: {}", e))
                })?;
                service_data.extend_from_slice(&encoded);
            }

            // monitoredProperty [4] — opening tag, then PropertyReference, then closing tag
            encode_opening_tag(&mut service_data, 4).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode opening tag 4: {}", e))
            })?;

            // PropertyReference: propertyIdentifier [0] + optional arrayIndex [1]
            let encoded = encode_context_unsigned(bacnet_property_id.into(), 0).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode property id: {}", e))
            })?;
            service_data.extend_from_slice(&encoded);

            encode_closing_tag(&mut service_data, 4).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode closing tag 4: {}", e))
            })?;

            // covIncrement [5] REAL OPTIONAL
            if let Some(increment) = cov_increment {
                encode_opening_tag(&mut service_data, 5).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode opening tag 5: {}", e))
                })?;
                encode_real(&mut service_data, increment).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode cov increment: {}", e))
                })?;
                encode_closing_tag(&mut service_data, 5).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode closing tag 5: {}", e))
                })?;
            }

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::SubscribeCOVProperty,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::SubscribeCOVProperty,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    /// Cancel a COV subscription by sending SubscribeCOV with lifetime=0.
    pub fn unsubscribe_cov(
        &self,
        device: DeviceId,
        object: ObjectId,
        subscriber_process_id: u32,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::service::SubscribeCovRequest;
            use type_conversion::to_bacnet_object_type;

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id =
                ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);

            // To cancel: SubscribeCOV with lifetime=0
            let sub_req =
                SubscribeCovRequest::with_lifetime(subscriber_process_id, bacnet_object_id, 0);

            let mut service_data = Vec::new();
            sub_req.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode unsubscribe: {}", e))
            })?;

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::SubscribeCOV,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::SubscribeCOV,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    /// Send DeviceCommunicationControl to a device (confirmed service 17).
    ///
    /// `time_duration`: seconds until communication is re-enabled (None = indefinite/0).
    /// `enable`: true = enable communication, false = disable.
    /// `password`: optional password if required by the device.
    pub fn device_communication_control(
        &self,
        device: DeviceId,
        time_duration: Option<u32>,
        enable: bool,
        password: Option<&str>,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
            use bacnet_rs::encoding::{
                encode_boolean, encode_character_string, encode_context_unsigned,
            };

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let mut service_data = Vec::new();

            // timeDuration [0] Unsigned32 OPTIONAL
            if let Some(duration) = time_duration {
                if duration > 0 {
                    encode_opening_tag(&mut service_data, 0).map_err(|e| {
                        ProtocolError::EncodingError(format!(
                            "Failed to encode opening tag 0: {}",
                            e
                        ))
                    })?;
                    let encoded = encode_context_unsigned(duration, 0).map_err(|e| {
                        ProtocolError::EncodingError(format!(
                            "Failed to encode time duration: {}",
                            e
                        ))
                    })?;
                    service_data.extend_from_slice(&encoded);
                    encode_closing_tag(&mut service_data, 0).map_err(|e| {
                        ProtocolError::EncodingError(format!(
                            "Failed to encode closing tag 0: {}",
                            e
                        ))
                    })?;
                }
            }

            // enable [1] BOOLEAN
            encode_opening_tag(&mut service_data, 1).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode opening tag 1: {}", e))
            })?;
            encode_boolean(&mut service_data, enable).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode enable flag: {}", e))
            })?;
            encode_closing_tag(&mut service_data, 1).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode closing tag 1: {}", e))
            })?;

            // password [2] CharacterString OPTIONAL
            if let Some(pwd) = password {
                encode_opening_tag(&mut service_data, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode opening tag 2: {}", e))
                })?;
                encode_character_string(&mut service_data, pwd).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode password: {}", e))
                })?;
                encode_closing_tag(&mut service_data, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode closing tag 2: {}", e))
                })?;
            }

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::DeviceCommunicationControl,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::DeviceCommunicationControl,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    /// Send ReinitializeDevice to a device (confirmed service 20).
    ///
    /// `state`: 0=coldstart, 1=warmstart, 2=startbackup, 3=endbackup,
    ///          4=startrestore, 5=endrestore, 6=activatesolar
    /// `password`: optional password if required by the device.
    pub fn reinitialize_device(
        &self,
        device: DeviceId,
        reinit_state: u32,
        password: Option<&str>,
    ) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
            use bacnet_rs::encoding::{encode_character_string, encode_context_enumerated};

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state_inner| {
                    let id = state_inner.next_invoke_id();
                    let seg = matches!(
                        state_inner.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state_inner.max_apdu, seg)
                })?;

            let mut service_data = Vec::new();

            // state [0] ENUMERATED
            let encoded = encode_context_enumerated(reinit_state, 0).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode reinit state: {}", e))
            })?;
            service_data.extend_from_slice(&encoded);

            // password [1] CharacterString OPTIONAL
            if let Some(pwd) = password {
                encode_opening_tag(&mut service_data, 1).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode opening tag 1: {}", e))
                })?;
                encode_character_string(&mut service_data, pwd).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode password: {}", e))
                })?;
                encode_closing_tag(&mut service_data, 1).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode closing tag 1: {}", e))
                })?;
            }

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::ReinitializeDevice,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::ReinitializeDevice,
                self.request_timeout,
                &self.stats,
            )?;

            Ok(())
        })
    }

    /// Send TimeSynchronization to a specific device (unconfirmed service 6).
    pub fn time_sync(&self, device: DeviceId) -> Result<(), ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::app::Apdu;
            use bacnet_rs::network::Npdu;
            use bacnet_rs::service::TimeSynchronizationRequest;

            let address = self.get_device_address(device)?;

            let sync = TimeSynchronizationRequest::now();
            let mut service_data = Vec::new();
            sync.encode(&mut service_data).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode TimeSync: {}", e))
            })?;

            let apdu = Apdu::UnconfirmedRequest {
                service_choice: UnconfirmedServiceChoice::TimeSynchronization,
                service_data,
            };

            let mut message = Npdu::new().encode();
            message.extend_from_slice(&apdu.encode());

            self.stats.record_send(message.len());
            self.transport.send(&address, &message)?;
            Ok(())
        })
    }

    /// Send UTC TimeSynchronization as a global broadcast (unconfirmed service 9).
    pub fn utc_time_sync_broadcast(&self) -> Result<(), ProtocolError> {
        use bacnet_rs::app::Apdu;
        use bacnet_rs::network::Npdu;
        use bacnet_rs::service::UtcTimeSynchronizationRequest;

        let utc_sync = UtcTimeSynchronizationRequest::now();
        let mut service_data = Vec::new();
        utc_sync.encode(&mut service_data).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode UTC TimeSync: {}", e))
        })?;

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: UnconfirmedServiceChoice::UtcTimeSynchronization,
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        self.stats.record_send(message.len());
        self.transport.broadcast(&message)?;
        Ok(())
    }

    // ==========================================================================
    // Phase A — Event Services (A6)
    // ==========================================================================

    /// Send ConfirmedEventNotification to a device (confirmed service 3).
    ///
    /// Requests event notification data from the specified device for the given object.
    ///
    /// # Arguments
    /// * `device` - Target device ID
    /// * `object_id` - Object to get event notification for
    /// * `event_state` - Optional event state filter (0 = no filter)
    /// * `initiating_process_id` - Process ID for the request
    ///
    /// # Returns
    /// A vector of event notification responses, one per requested object
    pub fn confirmed_event_notification(
        &self,
        device: DeviceId,
        object_id: ObjectId,
        event_state: Option<u32>,
        initiating_process_id: u32,
    ) -> Result<Vec<EventNotificationResponse>, ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
            use bacnet_rs::encoding::{
                encode_context_enumerated, encode_context_object_id, encode_context_unsigned,
            };
            use type_conversion::to_bacnet_object_type;

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id = ObjectIdentifier::new(
                to_bacnet_object_type(object_id.object_type),
                object_id.instance,
            );

            let mut service_data = Vec::new();

            // initiatingProcessIdentifier [0] Unsigned32
            let encoded = encode_context_unsigned(initiating_process_id, 0).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode process id: {}", e))
            })?;
            service_data.extend_from_slice(&encoded);

            // eventObjectIdentifier [1] BACnetObjectIdentifier
            let encoded_oid = encode_context_object_id(bacnet_object_id, 1).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode object id: {}", e))
            })?;
            service_data.extend_from_slice(&encoded_oid);

            // eventState [2] ENUMERATED OPTIONAL
            if let Some(es) = event_state {
                encode_opening_tag(&mut service_data, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode opening tag 2: {}", e))
                })?;
                let encoded = encode_context_enumerated(es, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode event state: {}", e))
                })?;
                service_data.extend_from_slice(&encoded);
                encode_closing_tag(&mut service_data, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode closing tag 2: {}", e))
                })?;
            }

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::ConfirmedEventNotification,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            let (response_data, _service_choice) = receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::ConfirmedEventNotification,
                self.request_timeout,
                &self.stats,
            )?;

            // Parse ConfirmedEventNotificationAck response
            parse_confirmed_event_notification_ack(&response_data)
        })
    }

    /// Send AcknowledgeAlarm to a device (confirmed service 8).
    ///
    /// Acknowledges an alarm on a specific object.
    ///
    /// # Arguments
    /// * `device` - Target device ID
    /// * `object_id` - Object containing the alarm to acknowledge
    /// * `acknowledged_from_state` - Optional boolean indicating the state from which the alarm is being acknowledged
    /// * `acknowledge_time` - Optional UTC time of acknowledgment
    /// * `processor_id` - Optional processor ID
    ///
    /// # Returns
    /// Response containing acknowledged state change info
    pub fn acknowledge_alarm(
        &self,
        device: DeviceId,
        object_id: ObjectId,
        acknowledged_from_state: Option<bool>,
        acknowledge_time: Option<String>,
        processor_id: Option<u32>,
    ) -> Result<AcknowledgeAlarmResponse, ProtocolError> {
        self.with_throttle(device, || {
            use bacnet_rs::encoding::advanced::context::{encode_closing_tag, encode_opening_tag};
            use bacnet_rs::encoding::{
                encode_boolean, encode_character_string, encode_context_object_id,
                encode_context_unsigned,
            };
            use type_conversion::to_bacnet_object_type;

            let address = self.get_device_address(device)?;

            let (invoke_id, max_apdu, seg_accepted) =
                self.get_device_state_mut(device, |state| {
                    let id = state.next_invoke_id();
                    let seg = matches!(
                        state.seg_supported,
                        bacnet_rs::object::Segmentation::Both
                            | bacnet_rs::object::Segmentation::Receive
                    );
                    (id, state.max_apdu, seg)
                })?;

            let bacnet_object_id = ObjectIdentifier::new(
                to_bacnet_object_type(object_id.object_type),
                object_id.instance,
            );

            let mut service_data = Vec::new();

            // ackRequestedFrom [0] BOOLEAN OPTIONAL
            if let Some(from_state) = acknowledged_from_state {
                encode_opening_tag(&mut service_data, 0).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode opening tag 0: {}", e))
                })?;
                encode_boolean(&mut service_data, from_state).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode boolean: {}", e))
                })?;
                encode_closing_tag(&mut service_data, 0).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode closing tag 0: {}", e))
                })?;
            }

            // eventObjectIdentifier [1] BACnetObjectIdentifier
            let encoded_oid = encode_context_object_id(bacnet_object_id, 1).map_err(|e| {
                ProtocolError::EncodingError(format!("Failed to encode object id: {}", e))
            })?;
            service_data.extend_from_slice(&encoded_oid);

            // acknowledgeTime [2] UTCTime OPTIONAL
            if let Some(time_str) = acknowledge_time {
                encode_opening_tag(&mut service_data, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode opening tag 2: {}", e))
                })?;
                encode_character_string(&mut service_data, &time_str).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode time: {}", e))
                })?;
                encode_closing_tag(&mut service_data, 2).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode closing tag 2: {}", e))
                })?;
            }

            // processorIdentifier [3] Unsigned32 OPTIONAL
            if let Some(proc_id) = processor_id {
                let encoded = encode_context_unsigned(proc_id, 3).map_err(|e| {
                    ProtocolError::EncodingError(format!("Failed to encode processor id: {}", e))
                })?;
                service_data.extend_from_slice(&encoded);
            }

            send_confirmed_request(
                &*self.transport,
                &address,
                ConfirmedServiceChoice::AcknowledgeAlarm,
                &service_data,
                invoke_id,
                seg_accepted,
                max_apdu,
                &self.stats,
            )?;

            let (response_data, _service_choice) = receive_response(
                &*self.transport,
                &address,
                invoke_id,
                ConfirmedServiceChoice::AcknowledgeAlarm,
                self.request_timeout,
                &self.stats,
            )?;

            parse_acknowledge_alarm_response(&response_data)
        })
    }

    /// Try to receive an unconfirmed event notification (unconfirmed service 12).
    /// Returns None on timeout.
    pub fn receive_event_notification(
        &self,
        timeout: Duration,
    ) -> Result<Option<EventNotificationInfo>, ProtocolError> {
        use bacnet_rs::network::Npdu;

        loop {
            let (_source_address, data) = match self.transport.receive(timeout) {
                Ok(v) => {
                    self.stats.record_receive(v.1.len());
                    v
                }
                Err(TransportError::Timeout) => return Ok(None),
                Err(e) => {
                    self.stats.record_error();
                    return Err(ProtocolError::TransportError(e));
                }
            };

            let mut offset = 0;
            if data.len() >= 4 && data[0] == 0x81 {
                offset = 4;
            }

            if data.len() <= offset {
                continue;
            }

            let (_, npdu_len) = match Npdu::decode(&data[offset..]) {
                Ok(v) => v,
                Err(_) => continue,
            };
            offset += npdu_len;

            if data.len() <= offset + 1 {
                continue;
            }

            // Check for UnconfirmedRequest APDU (0x10)
            if data[offset] != 0x10 {
                continue;
            }

            // Check service choice byte for UnconfirmedEventNotification = 12
            let service_byte = data[offset + 1];
            if service_byte != 12 {
                continue;
            }
            offset += 2;

            let payload = &data[offset..];
            match parse_event_notification_response(payload) {
                Ok(info) => return Ok(Some(info)),
                Err(_) => continue,
            }
        }
    }

    /// Try to receive a COV notification (unconfirmed). Returns None on timeout.
    pub fn receive_cov_notification(
        &self,
        timeout: Duration,
    ) -> Result<Option<CovNotification>, ProtocolError> {
        use bacnet_rs::network::Npdu;

        loop {
            let (source_address, data) = match self.transport.receive(timeout) {
                Ok(v) => {
                    self.stats.record_receive(v.1.len());
                    v
                }
                Err(TransportError::Timeout) => return Ok(None),
                Err(e) => {
                    self.stats.record_error();
                    return Err(ProtocolError::TransportError(e));
                }
            };

            let mut offset = 0;
            if data.len() >= 4 && data[0] == 0x81 {
                offset = 4;
            }

            if data.len() <= offset {
                continue;
            }

            let (_, npdu_len) = match Npdu::decode(&data[offset..]) {
                Ok(v) => v,
                Err(_) => continue,
            };
            offset += npdu_len;

            if data.len() <= offset + 1 {
                continue;
            }

            // Check for UnconfirmedRequest APDU (0x10)
            if data[offset] != 0x10 {
                continue;
            }

            // Check service choice byte
            // UnconfirmedCOVNotification = 2, UnconfirmedCOVNotificationMultiple = 11
            let service_byte = data[offset + 1];
            if service_byte != 2 && service_byte != 11 {
                continue;
            }
            offset += 2;

            let payload = &data[offset..];

            if service_byte == 2 {
                // Decode UnconfirmedCOVNotification
                let notification = decode_cov_notification(payload, source_address)?;
                return Ok(Some(notification));
            }
            // service_byte == 11: UnconfirmedCOVNotificationMultiple — not yet implemented
            eprintln!("UnconfirmedCOVNotificationMultiple (service 11) not yet supported");
            continue;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_rs::object::property_identifier::PropertyIdentifier;
    use bacnet_rs::object::{ObjectIdentifier, ObjectType as BacnetObjectType};
    use bacnet_rs::property::PropertyValue as BacnetPropertyValue;
    use bacnet_rs::service::ReadPropertyResponse;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[test]
    fn test_convert_bacnet_property_value_unsigned() {
        let bacnet_value = BacnetPropertyValue::Unsigned(42);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::Unsigned(v) => assert_eq!(v, 42),
            _ => panic!("Expected Unsigned value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_signed() {
        let bacnet_value = BacnetPropertyValue::Signed(-42);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::Integer(v) => assert_eq!(v, -42),
            _ => panic!("Expected Integer value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_real() {
        let bacnet_value = BacnetPropertyValue::Real(3.14);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::Real(v) => assert!((v - 3.14).abs() < 0.001),
            _ => panic!("Expected Real value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_boolean() {
        let bacnet_value = BacnetPropertyValue::Boolean(true);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::Boolean(v) => assert!(v),
            _ => panic!("Expected Boolean value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_string() {
        let bacnet_value = BacnetPropertyValue::CharacterString("test".to_string());
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::String(v) => assert_eq!(v, "test"),
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_enumerated() {
        let bacnet_value = BacnetPropertyValue::Enumerated(5);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::Enumerated(v) => assert_eq!(v, 5),
            _ => panic!("Expected Enumerated value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_object_identifier() {
        let obj_id = ObjectIdentifier::new(BacnetObjectType::AnalogInput, 123);
        let bacnet_value = BacnetPropertyValue::ObjectIdentifier(obj_id);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::ObjectIdentifier {
                object_type,
                instance,
            } => {
                assert_eq!(object_type, baccy_core::ObjectType::AnalogInput);
                assert_eq!(instance, 123);
            }
            _ => panic!("Expected ObjectIdentifier value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_octet_string() {
        let bacnet_value = BacnetPropertyValue::OctetString(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_ok());
        match result.unwrap() {
            PropertyValue::String(v) => assert_eq!(v, "deadbeef"),
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_convert_bacnet_property_value_null() {
        let bacnet_value = BacnetPropertyValue::Null;
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Null property values are not supported"));
    }

    #[test]
    fn test_convert_bacnet_property_value_unknown() {
        let bacnet_value = BacnetPropertyValue::Unknown(vec![0x01, 0x02]);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown property value type is not supported"));
    }

    #[test]
    fn test_read_property_response_parsing() {
        let obj_id = ObjectIdentifier::new(BacnetObjectType::AnalogInput, 1);
        let property_values = vec![
            BacnetPropertyValue::Real(72.5),
            BacnetPropertyValue::Unsigned(100),
        ];

        let response =
            ReadPropertyResponse::new(obj_id, PropertyIdentifier::PresentValue, property_values);

        assert!(!response.property_values.is_empty());
        assert_eq!(response.property_values.len(), 2);

        let first_value = &response.property_values[0];
        let converted = type_conversion::convert_bacnet_property_value(first_value);
        assert!(converted.is_ok());
        match converted.unwrap() {
            PropertyValue::Real(v) => assert!((v - 72.5).abs() < 0.001),
            _ => panic!("Expected Real value"),
        }
    }

    #[test]
    fn test_format_object_id() {
        let object = ObjectId {
            object_type: baccy_core::ObjectType::AnalogInput,
            instance: 42,
        };
        let formatted = format_object_id(&object);
        assert!(formatted.contains("AnalogInput"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn test_format_property_id() {
        let property = PropertyId::PresentValue;
        let formatted = format_property_id(&property);
        assert!(formatted.contains("PresentValue"));
    }

    #[test]
    fn test_invoke_id_wrapping() {
        let mut state = DeviceState {
            address: Address::MsTp { network: 0, mac: 1 },
            invoke_counter: 254,
            max_apdu: bacnet_rs::app::MaxApduSize::Up1476,
            seg_supported: bacnet_rs::object::Segmentation::NoSegmentation,
        };
        // 254 -> 255 -> 0 (wrap) -> 1
        assert_eq!(state.next_invoke_id(), 254);
        assert_eq!(state.next_invoke_id(), 255);
        assert_eq!(state.next_invoke_id(), 0);
        assert_eq!(state.next_invoke_id(), 1);
    }

    #[test]
    fn test_parse_max_apdu_from_iam_defaults() {
        let (max_apdu, seg) = parse_max_apdu_from_iam(&[]);
        assert_eq!(max_apdu, bacnet_rs::app::MaxApduSize::Up1476);
        assert_eq!(seg, bacnet_rs::object::Segmentation::NoSegmentation);
    }

    #[test]
    fn test_map_error_class() {
        assert_eq!(map_error_class(0), ErrorClass::Device);
        assert_eq!(map_error_class(2), ErrorClass::Property);
        assert_eq!(map_error_class(255), ErrorClass::Unknown(255));
    }

    #[test]
    fn test_map_error_code() {
        assert_eq!(map_error_code(0), ErrorCode::Other);
        assert_eq!(map_error_code(32), ErrorCode::UnknownProperty);
        assert_eq!(map_error_code(99), ErrorCode::Unknown(99));
    }

    #[test]
    fn test_cache_mstp_address() {
        use std::sync::Arc;
        use std::time::Duration;

        struct MockMstpTransport {
            responses: Arc<Mutex<VecDeque<(Address, Vec<u8>)>>>,
        }

        impl crate::Transport for MockMstpTransport {
            fn send(&self, _address: &Address, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }

            fn broadcast(&self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }

            fn receive(&self, _timeout: Duration) -> Result<(Address, Vec<u8>), TransportError> {
                let mut responses = self.responses.lock().unwrap();
                responses.pop_front().ok_or(TransportError::Timeout)
            }

            fn local_address(&self) -> Address {
                Address::MsTp { network: 0, mac: 5 }
            }
        }

        let device_id = 12345u32;
        let vendor_id = 42u16;

        let mut iam_data = vec![0x81, 0x0A, 0x00, 0x00];
        iam_data.extend_from_slice(&[0x01, 0x00]);
        iam_data.push(0x10);
        iam_data.push(0x00);

        iam_data.push(0xC4);
        let obj_id = (8u32 << 22) | device_id;
        iam_data.extend_from_slice(&obj_id.to_be_bytes());
        iam_data.push(0x21);
        iam_data.push(0x05);
        iam_data.push(0x91);
        iam_data.push(0x03);
        iam_data.push(0x22);
        iam_data.extend_from_slice(&vendor_id.to_be_bytes());

        let total_len = iam_data.len() as u16;
        iam_data[2] = (total_len >> 8) as u8;
        iam_data[3] = (total_len & 0xFF) as u8;

        let mstp_address = Address::MsTp {
            network: 0,
            mac: 42,
        };

        let mock_transport = Arc::new(MockMstpTransport {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![(
                mstp_address.clone(),
                iam_data,
            )]))),
        });

        let service = BacnetService::new(mock_transport, Duration::from_secs(1));

        let device = service.receive_iam(Duration::from_millis(100)).unwrap();
        assert_eq!(device.instance, device_id);

        let cached_address = service.get_device_address(device_id).unwrap();
        assert_eq!(cached_address, mstp_address);
    }
}
