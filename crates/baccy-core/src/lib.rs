use std::fmt;
use std::net::SocketAddr;

pub type DeviceId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    Ip(SocketAddr),
    MsTp { network: u16, mac: u8 },
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Ip(addr) => write!(f, "BACnet/IP {}", addr),
            Address::MsTp { network, mac } => write!(f, "MS/TP Network {} MAC {}", network, mac),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub instance: u32,
    pub name: String,
    pub vendor_id: u16,
    pub vendor_name: String,
    pub model_name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetObject {
    pub object_type: ObjectType,
    pub instance: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub object_type: ObjectType,
    pub instance: u32,
}

/// Extended BACnet object types (covering 40+ types from ANSI/ASHRAE 135)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    // Core types (already existed)
    AnalogInput,
    AnalogOutput,
    AnalogValue,
    BinaryInput,
    BinaryOutput,
    BinaryValue,
    Device,
    MultiStateInput,
    MultiStateOutput,
    MultiStateValue,
    // Extended types
    Calendar,
    Command,
    File,
    Group,
    EventEnrollment,
    Program,
    Schedule,
    Averaging,
    NotificationClass,
    TrendLog,
    LifeSafetyPoint,
    LifeSafetyZone,
    Loop,
    Accumulator,
    PulseConverter,
    EventLog,
    StructuredView,
    AccessDoor,
    CredentialDataInput,
    NetworkSecurity,
    BitStringValue,
    CharacterStringValue,
    DateTimeValue,
    IntegerValue,
    OctetStringValue,
    PositiveIntegerValue,
    TimeValue,
    NotificationForwarder,
    AlarmGroup,
    NetworkPort,
    ElevatorGroup,
    Escalator,
    Timer,
}

impl ObjectType {
    pub fn name(&self) -> &'static str {
        match self {
            ObjectType::AnalogInput => "Analog Input",
            ObjectType::AnalogOutput => "Analog Output",
            ObjectType::AnalogValue => "Analog Value",
            ObjectType::BinaryInput => "Binary Input",
            ObjectType::BinaryOutput => "Binary Output",
            ObjectType::BinaryValue => "Binary Value",
            ObjectType::Device => "Device",
            ObjectType::MultiStateInput => "Multi-State Input",
            ObjectType::MultiStateOutput => "Multi-State Output",
            ObjectType::MultiStateValue => "Multi-State Value",
            ObjectType::Calendar => "Calendar",
            ObjectType::Command => "Command",
            ObjectType::File => "File",
            ObjectType::Group => "Group",
            ObjectType::EventEnrollment => "Event Enrollment",
            ObjectType::Program => "Program",
            ObjectType::Schedule => "Schedule",
            ObjectType::Averaging => "Averaging",
            ObjectType::NotificationClass => "Notification Class",
            ObjectType::TrendLog => "Trend Log",
            ObjectType::LifeSafetyPoint => "Life Safety Point",
            ObjectType::LifeSafetyZone => "Life Safety Zone",
            ObjectType::Loop => "Loop",
            ObjectType::Accumulator => "Accumulator",
            ObjectType::PulseConverter => "Pulse Converter",
            ObjectType::EventLog => "Event Log",
            ObjectType::StructuredView => "Structured View",
            ObjectType::AccessDoor => "Access Door",
            ObjectType::CredentialDataInput => "Credential Data Input",
            ObjectType::NetworkSecurity => "Network Security",
            ObjectType::BitStringValue => "Bit-String Value",
            ObjectType::CharacterStringValue => "Character-String Value",
            ObjectType::DateTimeValue => "Date-Time Value",
            ObjectType::IntegerValue => "Integer Value",
            ObjectType::OctetStringValue => "Octet-String Value",
            ObjectType::PositiveIntegerValue => "Positive Integer Value",
            ObjectType::TimeValue => "Time Value",
            ObjectType::NotificationForwarder => "Notification Forwarder",
            ObjectType::AlarmGroup => "Alarm Group",
            ObjectType::NetworkPort => "Network Port",
            ObjectType::ElevatorGroup => "Elevator Group",
            ObjectType::Escalator => "Escalator",
            ObjectType::Timer => "Timer",
        }
    }

    pub fn from_debug_name(s: &str) -> Option<Self> {
        match s {
            "AnalogInput" => Some(Self::AnalogInput),
            "AnalogOutput" => Some(Self::AnalogOutput),
            "AnalogValue" => Some(Self::AnalogValue),
            "BinaryInput" => Some(Self::BinaryInput),
            "BinaryOutput" => Some(Self::BinaryOutput),
            "BinaryValue" => Some(Self::BinaryValue),
            "Device" => Some(Self::Device),
            "MultiStateInput" => Some(Self::MultiStateInput),
            "MultiStateOutput" => Some(Self::MultiStateOutput),
            "MultiStateValue" => Some(Self::MultiStateValue),
            "Calendar" => Some(Self::Calendar),
            "Command" => Some(Self::Command),
            "File" => Some(Self::File),
            "Group" => Some(Self::Group),
            "EventEnrollment" => Some(Self::EventEnrollment),
            "Program" => Some(Self::Program),
            "Schedule" => Some(Self::Schedule),
            "Averaging" => Some(Self::Averaging),
            "NotificationClass" => Some(Self::NotificationClass),
            "TrendLog" => Some(Self::TrendLog),
            "LifeSafetyPoint" => Some(Self::LifeSafetyPoint),
            "LifeSafetyZone" => Some(Self::LifeSafetyZone),
            "Loop" => Some(Self::Loop),
            "Accumulator" => Some(Self::Accumulator),
            "PulseConverter" => Some(Self::PulseConverter),
            "EventLog" => Some(Self::EventLog),
            "StructuredView" => Some(Self::StructuredView),
            "AccessDoor" => Some(Self::AccessDoor),
            "CredentialDataInput" => Some(Self::CredentialDataInput),
            "NetworkSecurity" => Some(Self::NetworkSecurity),
            "BitStringValue" => Some(Self::BitStringValue),
            "CharacterStringValue" => Some(Self::CharacterStringValue),
            "DateTimeValue" => Some(Self::DateTimeValue),
            "IntegerValue" => Some(Self::IntegerValue),
            "OctetStringValue" => Some(Self::OctetStringValue),
            "PositiveIntegerValue" => Some(Self::PositiveIntegerValue),
            "TimeValue" => Some(Self::TimeValue),
            "NotificationForwarder" => Some(Self::NotificationForwarder),
            "AlarmGroup" => Some(Self::AlarmGroup),
            "NetworkPort" => Some(Self::NetworkPort),
            "ElevatorGroup" => Some(Self::ElevatorGroup),
            "Escalator" => Some(Self::Escalator),
            "Timer" => Some(Self::Timer),
            _ => None,
        }
    }

    pub fn from_display_name(s: &str) -> Option<Self> {
        match s {
            "Analog Input" => Some(Self::AnalogInput),
            "Analog Output" => Some(Self::AnalogOutput),
            "Analog Value" => Some(Self::AnalogValue),
            "Binary Input" => Some(Self::BinaryInput),
            "Binary Output" => Some(Self::BinaryOutput),
            "Binary Value" => Some(Self::BinaryValue),
            "Device" => Some(Self::Device),
            "Multi-State Input" => Some(Self::MultiStateInput),
            "Multi-State Output" => Some(Self::MultiStateOutput),
            "Multi-State Value" => Some(Self::MultiStateValue),
            "Calendar" => Some(Self::Calendar),
            "Command" => Some(Self::Command),
            "File" => Some(Self::File),
            "Group" => Some(Self::Group),
            "Event Enrollment" => Some(Self::EventEnrollment),
            "Program" => Some(Self::Program),
            "Schedule" => Some(Self::Schedule),
            "Averaging" => Some(Self::Averaging),
            "Notification Class" => Some(Self::NotificationClass),
            "Trend Log" => Some(Self::TrendLog),
            "Life Safety Point" => Some(Self::LifeSafetyPoint),
            "Life Safety Zone" => Some(Self::LifeSafetyZone),
            "Loop" => Some(Self::Loop),
            "Accumulator" => Some(Self::Accumulator),
            "Pulse Converter" => Some(Self::PulseConverter),
            "Event Log" => Some(Self::EventLog),
            "Structured View" => Some(Self::StructuredView),
            "Access Door" => Some(Self::AccessDoor),
            "Credential Data Input" => Some(Self::CredentialDataInput),
            "Network Security" => Some(Self::NetworkSecurity),
            "Bit-String Value" => Some(Self::BitStringValue),
            "Character-String Value" => Some(Self::CharacterStringValue),
            "Date-Time Value" => Some(Self::DateTimeValue),
            "Integer Value" => Some(Self::IntegerValue),
            "Octet-String Value" => Some(Self::OctetStringValue),
            "Positive Integer Value" => Some(Self::PositiveIntegerValue),
            "Time Value" => Some(Self::TimeValue),
            "Notification Forwarder" => Some(Self::NotificationForwarder),
            "Alarm Group" => Some(Self::AlarmGroup),
            "Network Port" => Some(Self::NetworkPort),
            "Elevator Group" => Some(Self::ElevatorGroup),
            "Escalator" => Some(Self::Escalator),
            "Timer" => Some(Self::Timer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub id: PropertyId,
    pub name: String,
    pub value: PropertyValue,
    pub data_type: DataType,
    pub writable: bool,
}

/// Extended BACnet property identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyId {
    // Core types
    PresentValue,
    ObjectName,
    Description,
    Units,
    StatusFlags,
    OutOfService,
    Reliability,
    EventState,
    Priority,
    // Device/Vendor
    VendorName,
    ModelName,
    FirmwareRevision,
    AppSoftwareRevision,
    ProtocolVersion,
    ProtocolRevision,
    Location,
    ProfileName,
    // Lists/Capabilities
    SupportedObjectTypes,
    ObjectList,
    PropertyList,
    MaxApduLengthAccepted,
    SegmentationSupported,
    DeviceAddressBinding,
    DeviceType,
    MaxSegmentsAccepted,
    MaxInfoFrames,
    ObjectType,
    ListOfObjectProperty,
    // APDU/Timeout
    ApduSegmentTimeout,
    ApduTimeout,
    ApduLength,
    // TimeSync
    LocalDate,
    LocalTime,
    DaylightSavingsStatus,
    TimeSynchronizationRecipients,
    TimeSynchronizationInterval,
    // Backup/Restore
    BackupAndRestoreState,
    BackupPreparationTime,
    RestorePreparationTime,
    RestoreCompletionTime,
    LastRestoreTime,
    ConfigurationFiles,
    DatabaseRevision,
    ActiveCovSubscriptions,
    ActiveCovMultipleSubscriptions,
    // Alarming/Event
    AckedTransitions,
    CovIncrement,
    TimeDelay,
    NotificationClass,
    EventEnable,
    EventDetectionEnable,
    EventAlgorithmInhibit,
    EventAlgorithmInhibitRef,
    EventAlarmInhibited,
    NotifyType,
    EventTimeStamps,
    EventMessageTexts,
    EventMessageTextsConfig,
    PriorityForWriting,
    AlarmValue,
    AlarmValues,
    FaultValues,
    Setpoint,
    SetpointReference,
    // Trending/Logging
    LogDeviceObjectProperty,
    LoggingType,
    LogInterval,
    LogObject,
    LoggingRecord,
    RecordsSinceNotification,
    LastNotifyRecord,
    NotificationThreshold,
    NotificationThresholdCount,
    BufferSize,
    RecordCount,
    TotalRecordCount,
    StartTime,
    StopTime,
    LogBuffer,
    Enable,
    // Network
    NetworkNumber,
    NetworkNumberQuality,
    NetworkType,
    NetworkAccessSecurity,
    NetworkPriority,
    RoutingTable,
    RouterEntryDiscoveryTime,
    LinkSpeed,
    LinkSpeeds,
    LinkSpeedAutonegotiate,
    // StructuredView
    StructuredObjectList,
    SubordinateList,
    SubordinateNodeTypes,
    SubordinateAnnotations,
    SubordinateRelationships,
    SubordinateTags,
    // Other
    ProfileLocation,
    ValueSource,
    ValueSourceArray,
    ConstantValue,
    CommandTimeArray,
    DescriptionOfSchedule,
    PortLevel,
    PortNumber,
}

impl PropertyId {
    pub fn name(&self) -> &'static str {
        match self {
            // Core types
            PropertyId::PresentValue => "Present Value",
            PropertyId::ObjectName => "Object Name",
            PropertyId::Description => "Description",
            PropertyId::Units => "Units",
            PropertyId::StatusFlags => "Status Flags",
            PropertyId::OutOfService => "Out of Service",
            PropertyId::Reliability => "Reliability",
            PropertyId::EventState => "Event State",
            PropertyId::Priority => "Priority",
            // Device/Vendor
            PropertyId::VendorName => "Vendor Name",
            PropertyId::ModelName => "Model Name",
            PropertyId::FirmwareRevision => "Firmware Revision",
            PropertyId::AppSoftwareRevision => "Application Software Revision",
            PropertyId::ProtocolVersion => "Protocol Version",
            PropertyId::ProtocolRevision => "Protocol Revision",
            PropertyId::Location => "Location",
            PropertyId::ProfileName => "Profile Name",
            // Lists/Capabilities
            PropertyId::SupportedObjectTypes => "Supported Object Types",
            PropertyId::ObjectList => "Object List",
            PropertyId::PropertyList => "Property List",
            PropertyId::MaxApduLengthAccepted => "Max APDU Length Accepted",
            PropertyId::SegmentationSupported => "Segmentation Supported",
            PropertyId::DeviceAddressBinding => "Device Address Binding",
            PropertyId::DeviceType => "Device Type",
            PropertyId::MaxSegmentsAccepted => "Max Segments Accepted",
            PropertyId::MaxInfoFrames => "Max Info Frames",
            PropertyId::ObjectType => "Object Type",
            PropertyId::ListOfObjectProperty => "List of Object Property",
            // APDU/Timeout
            PropertyId::ApduSegmentTimeout => "APDU Segment Timeout",
            PropertyId::ApduTimeout => "APDU Timeout",
            PropertyId::ApduLength => "APDU Length",
            // TimeSync
            PropertyId::LocalDate => "Local Date",
            PropertyId::LocalTime => "Local Time",
            PropertyId::DaylightSavingsStatus => "Daylight Savings Status",
            PropertyId::TimeSynchronizationRecipients => "Time Synchronization Recipients",
            PropertyId::TimeSynchronizationInterval => "Time Synchronization Interval",
            // Backup/Restore
            PropertyId::BackupAndRestoreState => "Backup and Restore State",
            PropertyId::BackupPreparationTime => "Backup Preparation Time",
            PropertyId::RestorePreparationTime => "Restore Preparation Time",
            PropertyId::RestoreCompletionTime => "Restore Completion Time",
            PropertyId::LastRestoreTime => "Last Restore Time",
            PropertyId::ConfigurationFiles => "Configuration Files",
            PropertyId::DatabaseRevision => "Database Revision",
            PropertyId::ActiveCovSubscriptions => "Active COV Subscriptions",
            PropertyId::ActiveCovMultipleSubscriptions => "Active COV Multiple Subscriptions",
            // Alarming/Event
            PropertyId::AckedTransitions => "Acked Transitions",
            PropertyId::CovIncrement => "COV Increment",
            PropertyId::TimeDelay => "Time Delay",
            PropertyId::NotificationClass => "Notification Class",
            PropertyId::EventEnable => "Event Enable",
            PropertyId::EventDetectionEnable => "Event Detection Enable",
            PropertyId::EventAlgorithmInhibit => "Event Algorithm Inhibit",
            PropertyId::EventAlgorithmInhibitRef => "Event Algorithm Inhibit Ref",
            PropertyId::EventAlarmInhibited => "Event Alarm Inhibited",
            PropertyId::NotifyType => "Notify Type",
            PropertyId::EventTimeStamps => "Event Time Stamps",
            PropertyId::EventMessageTexts => "Event Message Texts",
            PropertyId::EventMessageTextsConfig => "Event Message Texts Config",
            PropertyId::PriorityForWriting => "Priority for Writing",
            PropertyId::AlarmValue => "Alarm Value",
            PropertyId::AlarmValues => "Alarm Values",
            PropertyId::FaultValues => "Fault Values",
            PropertyId::Setpoint => "Setpoint",
            PropertyId::SetpointReference => "Setpoint Reference",
            // Trending/Logging
            PropertyId::LogDeviceObjectProperty => "Log Device Object Property",
            PropertyId::LoggingType => "Logging Type",
            PropertyId::LogInterval => "Log Interval",
            PropertyId::LogObject => "Log Object",
            PropertyId::LoggingRecord => "Logging Record",
            PropertyId::RecordsSinceNotification => "Records Since Notification",
            PropertyId::LastNotifyRecord => "Last Notify Record",
            PropertyId::NotificationThreshold => "Notification Threshold",
            PropertyId::NotificationThresholdCount => "Notification Threshold Count",
            PropertyId::BufferSize => "Buffer Size",
            PropertyId::RecordCount => "Record Count",
            PropertyId::TotalRecordCount => "Total Record Count",
            PropertyId::StartTime => "Start Time",
            PropertyId::StopTime => "Stop Time",
            PropertyId::LogBuffer => "Log Buffer",
            PropertyId::Enable => "Enable",
            // Network
            PropertyId::NetworkNumber => "Network Number",
            PropertyId::NetworkNumberQuality => "Network Number Quality",
            PropertyId::NetworkType => "Network Type",
            PropertyId::NetworkAccessSecurity => "Network Access Security",
            PropertyId::NetworkPriority => "Network Priority",
            PropertyId::RoutingTable => "Routing Table",
            PropertyId::RouterEntryDiscoveryTime => "Router Entry Discovery Time",
            PropertyId::LinkSpeed => "Link Speed",
            PropertyId::LinkSpeeds => "Link Speeds",
            PropertyId::LinkSpeedAutonegotiate => "Link Speed Autonegotiate",
            // StructuredView
            PropertyId::StructuredObjectList => "Structured Object List",
            PropertyId::SubordinateList => "Subordinate List",
            PropertyId::SubordinateNodeTypes => "Subordinate Node Types",
            PropertyId::SubordinateAnnotations => "Subordinate Annotations",
            PropertyId::SubordinateRelationships => "Subordinate Relationships",
            PropertyId::SubordinateTags => "Subordinate Tags",
            // Other
            PropertyId::ProfileLocation => "Profile Location",
            PropertyId::ValueSource => "Value Source",
            PropertyId::ValueSourceArray => "Value Source Array",
            PropertyId::ConstantValue => "Constant Value",
            PropertyId::CommandTimeArray => "Command Time Array",
            PropertyId::DescriptionOfSchedule => "Description of Schedule",
            PropertyId::PortLevel => "Port Level",
            PropertyId::PortNumber => "Port Number",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Real(f32),
    Integer(i64),
    Unsigned(u64),
    Boolean(bool),
    String(String),
    Enumerated(u32),
    BitString(Vec<bool>),
    ObjectIdentifier { object_type: ObjectType, instance: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    Real,
    Integer,
    Unsigned,
    Boolean,
    CharacterString,
    Enumerated,
    BitString,
    ObjectIdentifier,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_mstp_creation() {
        let addr = Address::MsTp { network: 100, mac: 42 };
        match addr {
            Address::MsTp { network, mac } => {
                assert_eq!(network, 100);
                assert_eq!(mac, 42);
            }
            _ => panic!("Expected MsTp address"),
        }
    }

    #[test]
    fn test_address_mstp_display() {
        let addr = Address::MsTp { network: 100, mac: 42 };
        let display = format!("{}", addr);
        assert_eq!(display, "MS/TP Network 100 MAC 42");
    }

    #[test]
    fn test_address_ip_display() {
        let addr = Address::Ip("192.168.1.100:47808".parse().unwrap());
        let display = format!("{}", addr);
        assert_eq!(display, "BACnet/IP 192.168.1.100:47808");
    }

    #[test]
    fn test_address_debug_trait() {
        let addr = Address::MsTp { network: 100, mac: 42 };
        let debug = format!("{:?}", addr);
        assert!(debug.contains("MsTp"));
        assert!(debug.contains("100"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn test_address_clone_trait() {
        let addr1 = Address::MsTp { network: 100, mac: 42 };
        let addr2 = addr1.clone();
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_address_partial_eq_trait() {
        let addr1 = Address::MsTp { network: 100, mac: 42 };
        let addr2 = Address::MsTp { network: 100, mac: 42 };
        let addr3 = Address::MsTp { network: 100, mac: 43 };

        assert_eq!(addr1, addr2);
        assert_ne!(addr1, addr3);
    }

    #[test]
    fn test_object_type_names() {
        assert_eq!(ObjectType::AnalogInput.name(), "Analog Input");
        assert_eq!(ObjectType::Device.name(), "Device");
        assert_eq!(ObjectType::Calendar.name(), "Calendar");
        assert_eq!(ObjectType::Schedule.name(), "Schedule");
        assert_eq!(ObjectType::TrendLog.name(), "Trend Log");
        assert_eq!(ObjectType::Loop.name(), "Loop");
        assert_eq!(ObjectType::ElevatorGroup.name(), "Elevator Group");
        assert_eq!(ObjectType::Escalator.name(), "Escalator");
        assert_eq!(ObjectType::Timer.name(), "Timer");
    }

    #[test]
    fn test_property_id_names() {
        assert_eq!(PropertyId::VendorName.name(), "Vendor Name");
        assert_eq!(PropertyId::ModelName.name(), "Model Name");
        assert_eq!(PropertyId::FirmwareRevision.name(), "Firmware Revision");
        assert_eq!(PropertyId::ObjectList.name(), "Object List");
        assert_eq!(PropertyId::ObjectType.name(), "Object Type");
        assert_eq!(PropertyId::PropertyList.name(), "Property List");
        assert_eq!(PropertyId::DeviceType.name(), "Device Type");
        assert_eq!(PropertyId::Setpoint.name(), "Setpoint");
        assert_eq!(PropertyId::NetworkNumber.name(), "Network Number");
        assert_eq!(PropertyId::LoggingType.name(), "Logging Type");
        assert_eq!(PropertyId::EventMessageTexts.name(), "Event Message Texts");
    }

    #[test]
    fn test_property_id_unique() {
        use std::collections::HashSet;
        let mut names = HashSet::new();
        let mut ids = HashSet::new();
        // Test that each variant has a unique name and Debug repr is unique
        let variants = [
            PropertyId::PresentValue, PropertyId::ObjectName, PropertyId::Description,
            PropertyId::Units, PropertyId::StatusFlags, PropertyId::OutOfService,
            PropertyId::Reliability, PropertyId::EventState, PropertyId::Priority,
            PropertyId::VendorName, PropertyId::ModelName, PropertyId::FirmwareRevision,
            PropertyId::AppSoftwareRevision, PropertyId::ProtocolVersion, PropertyId::ProtocolRevision,
            PropertyId::Location, PropertyId::ProfileName,
            PropertyId::SupportedObjectTypes, PropertyId::ObjectList, PropertyId::PropertyList,
            PropertyId::MaxApduLengthAccepted, PropertyId::SegmentationSupported,
            PropertyId::DeviceAddressBinding, PropertyId::DeviceType, PropertyId::MaxSegmentsAccepted,
            PropertyId::MaxInfoFrames, PropertyId::ObjectType, PropertyId::ListOfObjectProperty,
            PropertyId::ApduSegmentTimeout, PropertyId::ApduTimeout, PropertyId::ApduLength,
            PropertyId::LocalDate, PropertyId::LocalTime, PropertyId::DaylightSavingsStatus,
            PropertyId::TimeSynchronizationRecipients, PropertyId::TimeSynchronizationInterval,
            PropertyId::BackupAndRestoreState, PropertyId::BackupPreparationTime,
            PropertyId::RestorePreparationTime, PropertyId::RestoreCompletionTime,
            PropertyId::LastRestoreTime, PropertyId::ConfigurationFiles,
            PropertyId::DatabaseRevision, PropertyId::ActiveCovSubscriptions,
            PropertyId::ActiveCovMultipleSubscriptions,
            PropertyId::AckedTransitions, PropertyId::CovIncrement, PropertyId::TimeDelay,
            PropertyId::NotificationClass, PropertyId::EventEnable,
            PropertyId::EventDetectionEnable, PropertyId::EventAlgorithmInhibit,
            PropertyId::EventAlgorithmInhibitRef, PropertyId::EventAlarmInhibited,
            PropertyId::NotifyType, PropertyId::EventTimeStamps,
            PropertyId::EventMessageTexts, PropertyId::EventMessageTextsConfig,
            PropertyId::PriorityForWriting, PropertyId::AlarmValue, PropertyId::AlarmValues,
            PropertyId::FaultValues, PropertyId::Setpoint, PropertyId::SetpointReference,
            PropertyId::LogDeviceObjectProperty, PropertyId::LoggingType, PropertyId::LogInterval,
            PropertyId::LogObject, PropertyId::LoggingRecord, PropertyId::RecordsSinceNotification,
            PropertyId::LastNotifyRecord, PropertyId::NotificationThreshold,
            PropertyId::NotificationThresholdCount, PropertyId::BufferSize, PropertyId::RecordCount,
            PropertyId::TotalRecordCount, PropertyId::StartTime, PropertyId::StopTime,
            PropertyId::LogBuffer, PropertyId::Enable,
            PropertyId::NetworkNumber, PropertyId::NetworkNumberQuality, PropertyId::NetworkType,
            PropertyId::NetworkAccessSecurity, PropertyId::NetworkPriority,
            PropertyId::RoutingTable, PropertyId::RouterEntryDiscoveryTime,
            PropertyId::LinkSpeed, PropertyId::LinkSpeeds, PropertyId::LinkSpeedAutonegotiate,
            PropertyId::StructuredObjectList, PropertyId::SubordinateList,
            PropertyId::SubordinateNodeTypes, PropertyId::SubordinateAnnotations,
            PropertyId::SubordinateRelationships, PropertyId::SubordinateTags,
            PropertyId::ProfileLocation, PropertyId::ValueSource, PropertyId::ValueSourceArray,
            PropertyId::ConstantValue, PropertyId::CommandTimeArray,
            PropertyId::DescriptionOfSchedule, PropertyId::PortLevel, PropertyId::PortNumber,
        ];
        for v in &variants {
            let debug = format!("{:?}", v);
            let name = v.name();
            // No duplicate debug names (variant names)
            assert!(ids.insert(debug), "Duplicate variant: {:?}", v);
            // Unique name() values
            assert!(names.insert(name), "Duplicate name(): {}", name);
        }
        // All 104 variants
        assert_eq!(variants.len(), 104);
    }

    #[test]
    fn test_invoke_id_wrapping() {
        // Test that the wrapping counter concept works
        let counter: u8 = 255;
        assert_eq!(counter.wrapping_add(1), 0);
        assert_eq!((0u8).wrapping_sub(1), 255);
    }
}
