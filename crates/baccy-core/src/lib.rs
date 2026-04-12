use std::net::SocketAddr;

/// Type alias for device identifiers (device instance number)
pub type DeviceId = u32;

/// Network address for BACnet communication
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    /// BACnet/IP address (UDP socket address)
    Ip(SocketAddr),
    /// MS/TP address (network number and MAC address)
    MsTp { network: u16, mac: Vec<u8> },
}

/// Represents a BACnet device on the network
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub instance: u32,
    pub name: String,
    pub vendor_id: u16,
    pub vendor_name: String,
    pub model_name: String,
    pub description: String,
}

/// Represents a BACnet object within a device
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetObject {
    pub object_type: ObjectType,
    pub instance: u32,
    pub name: String,
}

/// Unique identifier for a BACnet object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub object_type: ObjectType,
    pub instance: u32,
}

/// BACnet object types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
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
}

impl ObjectType {
    /// Get the human-readable name of the object type
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
        }
    }
}

/// Represents a property of a BACnet object
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub id: PropertyId,
    pub name: String,
    pub value: PropertyValue,
    pub data_type: DataType,
    pub writable: bool,
}

/// BACnet property identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyId {
    PresentValue,
    ObjectName,
    Description,
    Units,
    StatusFlags,
    OutOfService,
    Reliability,
    EventState,
    Priority,
}

impl PropertyId {
    /// Get the human-readable name of the property
    pub fn name(&self) -> &'static str {
        match self {
            PropertyId::PresentValue => "Present Value",
            PropertyId::ObjectName => "Object Name",
            PropertyId::Description => "Description",
            PropertyId::Units => "Units",
            PropertyId::StatusFlags => "Status Flags",
            PropertyId::OutOfService => "Out of Service",
            PropertyId::Reliability => "Reliability",
            PropertyId::EventState => "Event State",
            PropertyId::Priority => "Priority",
        }
    }
}

/// BACnet property values
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Real(f32),
    Integer(i32),
    Unsigned(u32),
    Boolean(bool),
    String(String),
    Enumerated(u32),
    BitString(Vec<bool>),
}

/// BACnet data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    Real,
    Integer,
    Unsigned,
    Boolean,
    CharacterString,
    Enumerated,
    BitString,
}
