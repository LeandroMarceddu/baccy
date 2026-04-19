use std::fmt;
use std::net::SocketAddr;

/// Type alias for device identifiers (device instance number)
pub type DeviceId = u32;

/// Network address for BACnet communication
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    /// BACnet/IP address (UDP socket address)
    Ip(SocketAddr),
    /// MS/TP address (network number and MAC address)
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
    Integer(i64),
    Unsigned(u64),
    Boolean(bool),
    String(String),
    Enumerated(u32),
    BitString(Vec<bool>),
    ObjectIdentifier { object_type: ObjectType, instance: u32 },
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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
    fn test_address_eq_trait() {
        let addr1 = Address::MsTp { network: 100, mac: 42 };
        let addr2 = Address::MsTp { network: 100, mac: 42 };
        
        // Eq trait is automatically derived, test via PartialEq
        assert!(addr1 == addr2);
    }

    #[test]
    fn test_address_hash_trait() {
        let addr1 = Address::MsTp { network: 100, mac: 42 };
        let addr2 = Address::MsTp { network: 100, mac: 42 };
        let addr3 = Address::MsTp { network: 100, mac: 43 };
        
        let mut set = HashSet::new();
        set.insert(addr1.clone());
        
        // Same address should be found in set
        assert!(set.contains(&addr2));
        
        // Different address should not be found
        assert!(!set.contains(&addr3));
    }

    #[test]
    fn test_address_mstp_mac_range() {
        // Test boundary values for MAC address (u8: 0-255)
        let addr_min = Address::MsTp { network: 0, mac: 0 };
        let addr_max = Address::MsTp { network: 65535, mac: 255 };
        
        assert_eq!(format!("{}", addr_min), "MS/TP Network 0 MAC 0");
        assert_eq!(format!("{}", addr_max), "MS/TP Network 65535 MAC 255");
    }
}
