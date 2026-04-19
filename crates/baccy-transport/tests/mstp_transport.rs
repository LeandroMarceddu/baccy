// Integration tests for MS/TP transport

use baccy_transport::{BacnetMstpTransport, TransportError};

#[test]
fn test_invalid_baud_rate() {
    // Test that invalid baud rates are rejected
    let result = BacnetMstpTransport::new("/dev/ttyUSB0", 57600, 5);
    assert!(result.is_err());
    
    if let Err(TransportError::BindFailed(e)) = result {
        let msg = e.to_string();
        assert!(msg.contains("Unsupported baud rate"));
        assert!(msg.contains("9600, 19200, 38400, 76800, 115200"));
    } else {
        panic!("Expected BindFailed error");
    }
}

#[test]
fn test_invalid_mac_address() {
    // Test that MAC addresses > 127 are rejected for master nodes
    let result = BacnetMstpTransport::new("/dev/ttyUSB0", 38400, 128);
    assert!(result.is_err());
    
    if let Err(TransportError::BindFailed(e)) = result {
        let msg = e.to_string();
        assert!(msg.contains("Invalid MAC address"));
        assert!(msg.contains("0-127"));
    } else {
        panic!("Expected BindFailed error");
    }
}

#[test]
fn test_valid_baud_rates() {
    // Test that all supported baud rates are accepted (will fail on port open, but validation passes)
    let supported_rates = [9600, 19200, 38400, 76800, 115200];
    
    for rate in supported_rates {
        let result = BacnetMstpTransport::new("/dev/nonexistent_port", rate, 5);
        // Should fail on port open, not on baud rate validation
        if let Err(TransportError::BindFailed(e)) = result {
            let msg = e.to_string();
            // Should NOT contain "Unsupported baud rate"
            assert!(!msg.contains("Unsupported baud rate"), 
                "Baud rate {} should be supported but got error: {}", rate, msg);
        }
    }
}

#[test]
fn test_valid_mac_addresses() {
    // Test that MAC addresses 0-127 are accepted (will fail on port open, but validation passes)
    for mac in [0, 1, 63, 127] {
        let result = BacnetMstpTransport::new("/dev/nonexistent_port", 38400, mac);
        // Should fail on port open, not on MAC validation
        if let Err(TransportError::BindFailed(e)) = result {
            let msg = e.to_string();
            // Should NOT contain "Invalid MAC address"
            assert!(!msg.contains("Invalid MAC address"), 
                "MAC address {} should be valid but got error: {}", mac, msg);
        }
    }
}

#[test]
fn test_port_not_found_error_message() {
    // Test that port not found errors provide helpful messages
    let result = BacnetMstpTransport::new("/dev/nonexistent_port_12345", 38400, 5);
    assert!(result.is_err());
    
    if let Err(TransportError::BindFailed(e)) = result {
        let msg = e.to_string();
        assert!(msg.contains("not found") || msg.contains("No such file"));
    } else {
        panic!("Expected BindFailed error");
    }
}

// Note: Tests for send() method with actual serial port communication
// would require mock serial ports or hardware. The send() method is tested
// indirectly through the frame encoding tests and will be tested in
// integration tests with real hardware or mock serial ports.
//
// The key behaviors tested elsewhere:
// - Frame encoding (frame.rs tests)
// - CRC calculation (crc_validation.rs tests)
// - Address type checking (would require mock serial port)
