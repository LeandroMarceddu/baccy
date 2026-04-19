// Transport configuration types for BACnet communication

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Configuration for BACnet transport layer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransportConfig {
    /// BACnet/IP transport configuration
    Ip {
        /// Socket address to bind to (IP:port)
        bind_addr: SocketAddr,
    },
    /// MS/TP transport configuration
    #[serde(rename = "mstp")]
    MsTp {
        /// Serial port name (e.g., "/dev/ttyUSB0", "COM3")
        port_name: String,
        /// Baud rate (9600, 19200, 38400, 76800)
        baud_rate: u32,
        /// Local MAC address (0-127 for master nodes)
        local_mac: u8,
    },
}

impl TransportConfig {
    /// Create a new BACnet/IP transport configuration
    pub fn new_ip(bind_addr: SocketAddr) -> Self {
        Self::Ip { bind_addr }
    }

    /// Create a new MS/TP transport configuration
    pub fn new_mstp(port_name: String, baud_rate: u32, local_mac: u8) -> Self {
        Self::MsTp {
            port_name,
            baud_rate,
            local_mac,
        }
    }

    /// Get a human-readable description of the transport configuration
    pub fn description(&self) -> String {
        match self {
            TransportConfig::Ip { bind_addr } => {
                format!("BACnet/IP ({})", bind_addr)
            }
            TransportConfig::MsTp {
                port_name,
                baud_rate,
                local_mac,
            } => {
                format!("MS/TP ({} @ {} bps, MAC {})", port_name, baud_rate, local_mac)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config_ip_creation() {
        let addr: SocketAddr = "192.168.1.100:47808".parse().unwrap();
        let config = TransportConfig::new_ip(addr);
        
        match config {
            TransportConfig::Ip { bind_addr } => {
                assert_eq!(bind_addr, addr);
            }
            _ => panic!("Expected Ip variant"),
        }
    }

    #[test]
    fn test_transport_config_mstp_creation() {
        let config = TransportConfig::new_mstp("/dev/ttyUSB0".to_string(), 38400, 42);
        
        match config {
            TransportConfig::MsTp {
                port_name,
                baud_rate,
                local_mac,
            } => {
                assert_eq!(port_name, "/dev/ttyUSB0");
                assert_eq!(baud_rate, 38400);
                assert_eq!(local_mac, 42);
            }
            _ => panic!("Expected MsTp variant"),
        }
    }

    #[test]
    fn test_transport_config_ip_description() {
        let addr: SocketAddr = "192.168.1.100:47808".parse().unwrap();
        let config = TransportConfig::new_ip(addr);
        let desc = config.description();
        
        assert_eq!(desc, "BACnet/IP (192.168.1.100:47808)");
    }

    #[test]
    fn test_transport_config_mstp_description() {
        let config = TransportConfig::new_mstp("/dev/ttyUSB0".to_string(), 38400, 42);
        let desc = config.description();
        
        assert_eq!(desc, "MS/TP (/dev/ttyUSB0 @ 38400 bps, MAC 42)");
    }

    #[test]
    fn test_transport_config_serialization() {
        let config = TransportConfig::new_ip("192.168.1.100:47808".parse().unwrap());
        let json = serde_json::to_string(&config).unwrap();
        
        assert!(json.contains("\"type\":\"ip\""));
        assert!(json.contains("bind_addr"));
    }

    #[test]
    fn test_transport_config_deserialization() {
        let json = r#"{"type":"ip","bind_addr":"192.168.1.100:47808"}"#;
        let config: TransportConfig = serde_json::from_str(json).unwrap();
        
        match config {
            TransportConfig::Ip { bind_addr } => {
                assert_eq!(bind_addr.to_string(), "192.168.1.100:47808");
            }
            _ => panic!("Expected Ip variant"),
        }
    }

    #[test]
    fn test_transport_config_mstp_serialization() {
        let config = TransportConfig::new_mstp("/dev/ttyUSB0".to_string(), 38400, 42);
        let json = serde_json::to_string(&config).unwrap();
        
        assert!(json.contains("\"type\":\"mstp\""));
        assert!(json.contains("port_name"));
        assert!(json.contains("baud_rate"));
        assert!(json.contains("local_mac"));
    }

    #[test]
    fn test_transport_config_mstp_deserialization() {
        let json = r#"{"type":"mstp","port_name":"/dev/ttyUSB0","baud_rate":38400,"local_mac":42}"#;
        let config: TransportConfig = serde_json::from_str(json).unwrap();
        
        match config {
            TransportConfig::MsTp {
                port_name,
                baud_rate,
                local_mac,
            } => {
                assert_eq!(port_name, "/dev/ttyUSB0");
                assert_eq!(baud_rate, 38400);
                assert_eq!(local_mac, 42);
            }
            _ => panic!("Expected MsTp variant"),
        }
    }
}
