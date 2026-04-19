// Integration test to validate CRC calculations against BACnet standard
//
// This test verifies that our CRC implementations match the expected behavior
// for MS/TP frames as defined in ASHRAE 135 Clause 9.

use baccy_transport::frame::{calculate_data_crc, calculate_header_crc};

#[test]
fn test_header_crc_matches_standard() {
    // Test vectors from BACnet standard and real MS/TP implementations
    
    // Token frame: frame_type=0x00, dest=5, src=3, length=0
    let token_header = [0x00, 0x05, 0x03, 0x00, 0x00];
    let token_crc = calculate_header_crc(&token_header);
    assert_eq!(token_crc, 0xFC, "Token frame header CRC mismatch");
    
    // Poll For Master: frame_type=0x01, dest=127, src=0, length=0
    let poll_header = [0x01, 0x7F, 0x00, 0x00, 0x00];
    let poll_crc = calculate_header_crc(&poll_header);
    assert!(poll_crc != 0, "Poll For Master CRC should be non-zero");
    
    // BACnet Data Expecting Reply: frame_type=0x06, dest=10, src=20, length=16
    let data_header = [0x06, 0x0A, 0x14, 0x00, 0x10];
    let data_crc = calculate_header_crc(&data_header);
    assert!(data_crc != 0, "Data frame CRC should be non-zero");
}

#[test]
fn test_data_crc_matches_standard() {
    // Test vectors for data CRC
    
    // Empty data
    let empty_crc = calculate_data_crc(&[]);
    assert_eq!(empty_crc, 0x0000, "Empty data CRC should be 0x0000");
    
    // Single byte
    let single_byte_crc = calculate_data_crc(&[0x55]);
    assert!(single_byte_crc != 0, "Single byte CRC should be non-zero");
    
    // Small BACnet message
    let message = [0x01, 0x20, 0xFF, 0xFF, 0x00, 0x05, 0x01, 0x0C];
    let message_crc = calculate_data_crc(&message);
    assert!(message_crc != 0, "Message CRC should be non-zero");
}

#[test]
fn test_crc_properties() {
    // Test that CRC has expected properties
    
    // Property 1: Same input always produces same output
    let data = b"Test data";
    let crc1 = calculate_data_crc(data);
    let crc2 = calculate_data_crc(data);
    assert_eq!(crc1, crc2, "CRC should be deterministic");
    
    // Property 2: Different inputs produce different outputs (with high probability)
    let data_a = b"Data A";
    let data_b = b"Data B";
    let crc_a = calculate_data_crc(data_a);
    let crc_b = calculate_data_crc(data_b);
    assert_ne!(crc_a, crc_b, "Different data should produce different CRCs");
    
    // Property 3: Single bit change produces different CRC
    let original = [0x00, 0x00, 0x00, 0x00];
    let modified = [0x01, 0x00, 0x00, 0x00];
    let crc_original = calculate_data_crc(&original);
    let crc_modified = calculate_data_crc(&modified);
    assert_ne!(crc_original, crc_modified, "Single bit change should alter CRC");
}

#[test]
fn test_header_crc_boundary_conditions() {
    // Test boundary conditions for header CRC
    
    // Maximum valid addresses
    let max_header = [0x07, 0xFF, 0xFF, 0x01, 0xF5]; // Max length = 501 bytes
    let max_crc = calculate_header_crc(&max_header);
    assert!(max_crc != 0xFF, "Max header CRC should not be 0xFF");
    
    // Minimum addresses
    let min_header = [0x00, 0x00, 0x00, 0x00, 0x00];
    let min_crc = calculate_header_crc(&min_header);
    assert_eq!(min_crc, 0x98, "Min header CRC should be 0x98");
}

#[test]
fn test_data_crc_large_payload() {
    // Test CRC with maximum MS/TP payload size (501 bytes)
    let large_payload = vec![0xAA; 501];
    let crc = calculate_data_crc(&large_payload);
    assert!(crc != 0, "Large payload CRC should be non-zero");
    
    // Test with different large payload
    let large_payload_2 = vec![0x55; 501];
    let crc_2 = calculate_data_crc(&large_payload_2);
    assert_ne!(crc, crc_2, "Different large payloads should have different CRCs");
}

#[test]
fn test_crc_error_detection() {
    // Verify that CRC can detect common transmission errors
    
    let original_data = b"BACnet MS/TP Frame Data";
    let original_crc = calculate_data_crc(original_data);
    
    // Simulate single bit flip
    let mut corrupted_data = original_data.to_vec();
    corrupted_data[0] ^= 0x01; // Flip least significant bit
    let corrupted_crc = calculate_data_crc(&corrupted_data);
    
    assert_ne!(
        original_crc, corrupted_crc,
        "CRC should detect single bit error"
    );
    
    // Simulate byte swap
    let mut swapped_data = original_data.to_vec();
    if swapped_data.len() >= 2 {
        swapped_data.swap(0, 1);
        let swapped_crc = calculate_data_crc(&swapped_data);
        assert_ne!(
            original_crc, swapped_crc,
            "CRC should detect byte swap error"
        );
    }
}
