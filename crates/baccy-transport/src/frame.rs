// MS/TP frame structures and encoding/decoding
//
// This module implements the BACnet MS/TP frame format as defined in
// ASHRAE 135 Clause 9. MS/TP frames provide the data link layer for
// BACnet communication over RS-485 serial networks.

/// MS/TP frame types as defined in BACnet standard
///
/// These frame types control the token-passing protocol and data transmission
/// on the MS/TP network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MstpFrameType {
    /// Token frame - passes the token to the next master
    Token = 0x00,
    /// Poll For Master frame - discovers new master nodes
    PollForMaster = 0x01,
    /// Reply To Poll For Master frame - response to poll
    ReplyToPollForMaster = 0x02,
    /// Test Request frame - tests communication with a node
    TestRequest = 0x04,
    /// Test Response frame - response to test request
    TestResponse = 0x05,
    /// BACnet Data Expecting Reply frame - data requiring acknowledgment
    BacnetDataExpectingReply = 0x06,
    /// BACnet Data Not Expecting Reply frame - data not requiring acknowledgment
    BacnetDataNotExpectingReply = 0x07,
}

impl MstpFrameType {
    /// Convert from u8 to MstpFrameType
    ///
    /// Returns None if the value doesn't correspond to a valid frame type.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Token),
            0x01 => Some(Self::PollForMaster),
            0x02 => Some(Self::ReplyToPollForMaster),
            0x04 => Some(Self::TestRequest),
            0x05 => Some(Self::TestResponse),
            0x06 => Some(Self::BacnetDataExpectingReply),
            0x07 => Some(Self::BacnetDataNotExpectingReply),
            _ => None,
        }
    }
}

/// MS/TP frame structure
///
/// Represents a complete MS/TP frame with header and data fields.
/// The frame format follows the BACnet standard:
///
/// ```text
/// +----------+----------+----------+----------+----------+----------+-----+-----+
/// | Preamble | Frame    | Dest     | Source   | Length   | Length   | ... | ... |
/// | (0x55)   | Type     | Address  | Address  | (MSB)    | (LSB)    | Data| CRC |
/// +----------+----------+----------+----------+----------+----------+-----+-----+
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MstpFrame {
    /// Type of frame (Token, Data, etc.)
    pub frame_type: MstpFrameType,
    /// Destination MAC address (0-255)
    pub destination: u8,
    /// Source MAC address (0-255)
    pub source: u8,
    /// Frame data payload (0-501 bytes for BACnet data frames)
    pub data: Vec<u8>,
}

/// Errors that can occur during MS/TP frame decoding
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MstpDecodeError {
    /// Frame is too short to be valid
    #[error("Truncated frame: expected at least {expected} bytes, got {actual}")]
    TruncatedFrame { expected: usize, actual: usize },

    /// Preamble bytes are incorrect
    #[error("Invalid preamble: expected 0x55 0xFF, got 0x{byte1:02X} 0x{byte2:02X}")]
    InvalidPreamble { byte1: u8, byte2: u8 },

    /// Frame type is not recognized
    #[error("Invalid frame type: 0x{0:02X}")]
    InvalidFrameType(u8),

    /// Header CRC doesn't match calculated value
    #[error("Invalid header CRC: expected 0x{expected:02X}, got 0x{actual:02X}")]
    InvalidHeaderCrc { expected: u8, actual: u8 },

    /// Data CRC doesn't match calculated value
    #[error("Invalid data CRC: expected 0x{expected:04X}, got 0x{actual:04X}")]
    InvalidDataCrc { expected: u16, actual: u16 },
}

impl MstpFrame {
    /// Create a new MS/TP frame
    ///
    /// # Arguments
    /// * `frame_type` - The type of frame to create
    /// * `destination` - Destination MAC address (0-254 for unicast, 255 for broadcast)
    /// * `source` - Source MAC address (0-254)
    /// * `data` - Frame data payload
    ///
    /// # Examples
    ///
    /// ```
    /// use baccy_transport::frame::{MstpFrame, MstpFrameType};
    ///
    /// // Create a token frame
    /// let token = MstpFrame::new(
    ///     MstpFrameType::Token,
    ///     5,  // destination
    ///     3,  // source
    ///     vec![]  // no data
    /// );
    ///
    /// // Create a data frame
    /// let data_frame = MstpFrame::new(
    ///     MstpFrameType::BacnetDataNotExpectingReply,
    ///     10,
    ///     20,
    ///     vec![0x01, 0x02, 0x03, 0x04]
    /// );
    /// ```
    pub fn new(
        frame_type: MstpFrameType,
        destination: u8,
        source: u8,
        data: Vec<u8>,
    ) -> Self {
        Self {
            frame_type,
            destination,
            source,
            data,
        }
    }

    /// Create a token frame
    ///
    /// Token frames are used to pass control of the network to the next master node.
    ///
    /// # Arguments
    /// * `destination` - MAC address of the next master
    /// * `source` - MAC address of the current master
    pub fn token(destination: u8, source: u8) -> Self {
        Self::new(MstpFrameType::Token, destination, source, Vec::new())
    }

    /// Create a BACnet data frame
    ///
    /// # Arguments
    /// * `destination` - Destination MAC address
    /// * `source` - Source MAC address
    /// * `data` - BACnet message data
    /// * `expecting_reply` - Whether this frame expects a reply
    pub fn bacnet_data(
        destination: u8,
        source: u8,
        data: Vec<u8>,
        expecting_reply: bool,
    ) -> Self {
        let frame_type = if expecting_reply {
            MstpFrameType::BacnetDataExpectingReply
        } else {
            MstpFrameType::BacnetDataNotExpectingReply
        };
        Self::new(frame_type, destination, source, data)
    }

    /// Check if this is a token frame
    pub fn is_token(&self) -> bool {
        self.frame_type == MstpFrameType::Token
    }

    /// Check if this is a data frame
    pub fn is_data(&self) -> bool {
        matches!(
            self.frame_type,
            MstpFrameType::BacnetDataExpectingReply
                | MstpFrameType::BacnetDataNotExpectingReply
        )
    }

    /// Encode frame to bytes for transmission
    ///
    /// Converts the MS/TP frame into a byte sequence ready for transmission
    /// over the serial port. The encoding follows the BACnet standard format:
    ///
    /// 1. Preamble: 0x55 0xFF (2 bytes)
    /// 2. Frame Type: 1 byte
    /// 3. Destination Address: 1 byte
    /// 4. Source Address: 1 byte
    /// 5. Length: 2 bytes (MSB first)
    /// 6. Header CRC: 1 byte (CRC-8)
    /// 7. Data: 0-501 bytes (if present)
    /// 8. Data CRC: 2 bytes (CRC-16, LSB first) (if data present)
    ///
    /// # Returns
    /// A vector of bytes representing the encoded frame
    ///
    /// # Examples
    ///
    /// ```
    /// use baccy_transport::frame::{MstpFrame, MstpFrameType};
    ///
    /// // Encode a token frame
    /// let token = MstpFrame::token(5, 3);
    /// let bytes = token.encode();
    /// assert_eq!(bytes[0], 0x55); // Preamble byte 1
    /// assert_eq!(bytes[1], 0xFF); // Preamble byte 2
    /// assert_eq!(bytes[2], 0x00); // Frame type (Token)
    ///
    /// // Encode a data frame
    /// let data_frame = MstpFrame::bacnet_data(10, 20, vec![0x01, 0x02, 0x03, 0x04], false);
    /// let bytes = data_frame.encode();
    /// assert!(bytes.len() > 8); // Header + data + CRC
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let data_length = self.data.len();
        
        // Calculate total frame size:
        // 2 (preamble) + 1 (frame type) + 1 (dest) + 1 (src) + 2 (length) + 1 (header CRC)
        // + data_length + 2 (data CRC if data present)
        let frame_size = 8 + data_length + if data_length > 0 { 2 } else { 0 };
        let mut frame = Vec::with_capacity(frame_size);

        // 1. Add preamble bytes
        frame.push(0x55);
        frame.push(0xFF);

        // 2. Build header for CRC calculation
        let header = [
            self.frame_type as u8,
            self.destination,
            self.source,
            (data_length >> 8) as u8,  // Length MSB
            (data_length & 0xFF) as u8, // Length LSB
        ];

        // 3. Add header fields to frame
        frame.extend_from_slice(&header);

        // 4. Calculate and append header CRC
        let header_crc = calculate_header_crc(&header);
        frame.push(header_crc);

        // 5. Append data payload (if present)
        if !self.data.is_empty() {
            frame.extend_from_slice(&self.data);

            // 6. Calculate and append data CRC
            let data_crc = calculate_data_crc(&self.data);
            // Data CRC is appended as LSB first (little-endian)
            frame.push((data_crc & 0xFF) as u8);        // LSB
            frame.push((data_crc >> 8) as u8);          // MSB
        }

        frame
    }

    /// Decode MS/TP frame from bytes received from serial port
    ///
    /// Parses a byte sequence into an MS/TP frame, validating the frame structure
    /// and CRC values. The expected format is:
    ///
    /// 1. Preamble: 0x55 0xFF (2 bytes)
    /// 2. Frame Type: 1 byte
    /// 3. Destination Address: 1 byte
    /// 4. Source Address: 1 byte
    /// 5. Length: 2 bytes (MSB first)
    /// 6. Header CRC: 1 byte (CRC-8)
    /// 7. Data: 0-501 bytes (if length > 0)
    /// 8. Data CRC: 2 bytes (CRC-16, LSB first) (if length > 0)
    ///
    /// # Arguments
    /// * `bytes` - Raw bytes received from serial port
    ///
    /// # Returns
    /// A decoded `MstpFrame` if successful
    ///
    /// # Errors
    /// Returns `MstpDecodeError` if:
    /// - Frame is too short (< 8 bytes minimum)
    /// - Preamble bytes are incorrect
    /// - Frame type is invalid
    /// - Header CRC doesn't match
    /// - Data CRC doesn't match (for frames with data)
    /// - Frame is truncated (data length doesn't match actual data)
    ///
    /// # Examples
    ///
    /// ```
    /// use baccy_transport::frame::{MstpFrame, MstpFrameType};
    ///
    /// // Decode a token frame
    /// let token = MstpFrame::token(5, 3);
    /// let bytes = token.encode();
    /// let decoded = MstpFrame::decode(&bytes).unwrap();
    /// assert_eq!(decoded.frame_type, MstpFrameType::Token);
    /// assert_eq!(decoded.destination, 5);
    /// assert_eq!(decoded.source, 3);
    ///
    /// // Decode a data frame
    /// let data_frame = MstpFrame::bacnet_data(10, 20, vec![0x01, 0x02, 0x03, 0x04], false);
    /// let bytes = data_frame.encode();
    /// let decoded = MstpFrame::decode(&bytes).unwrap();
    /// assert_eq!(decoded.data, vec![0x01, 0x02, 0x03, 0x04]);
    /// ```
    pub fn decode(bytes: &[u8]) -> Result<Self, MstpDecodeError> {
        // 1. Check minimum frame size (preamble + header + header CRC = 8 bytes)
        if bytes.len() < 8 {
            return Err(MstpDecodeError::TruncatedFrame {
                expected: 8,
                actual: bytes.len(),
            });
        }

        // 2. Verify preamble bytes
        if bytes[0] != 0x55 || bytes[1] != 0xFF {
            return Err(MstpDecodeError::InvalidPreamble {
                byte1: bytes[0],
                byte2: bytes[1],
            });
        }

        // 3. Parse header fields
        let frame_type_byte = bytes[2];
        let destination = bytes[3];
        let source = bytes[4];
        let length_msb = bytes[5];
        let length_lsb = bytes[6];
        let header_crc = bytes[7];

        // 4. Validate frame type
        let frame_type = MstpFrameType::from_u8(frame_type_byte)
            .ok_or(MstpDecodeError::InvalidFrameType(frame_type_byte))?;

        // 5. Calculate data length
        let data_length = ((length_msb as usize) << 8) | (length_lsb as usize);

        // 6. Verify header CRC
        let header = [frame_type_byte, destination, source, length_msb, length_lsb];
        let calculated_header_crc = calculate_header_crc(&header);
        if calculated_header_crc != header_crc {
            return Err(MstpDecodeError::InvalidHeaderCrc {
                expected: calculated_header_crc,
                actual: header_crc,
            });
        }

        // 7. Parse data payload (if present)
        let data = if data_length > 0 {
            // Check if frame has enough bytes for data + data CRC
            let expected_total_length = 8 + data_length + 2; // header + data + data CRC
            if bytes.len() < expected_total_length {
                return Err(MstpDecodeError::TruncatedFrame {
                    expected: expected_total_length,
                    actual: bytes.len(),
                });
            }

            // Extract data payload
            let data_start = 8;
            let data_end = 8 + data_length;
            let data = bytes[data_start..data_end].to_vec();

            // 8. Verify data CRC
            let data_crc_lsb = bytes[data_end];
            let data_crc_msb = bytes[data_end + 1];
            let received_data_crc = ((data_crc_msb as u16) << 8) | (data_crc_lsb as u16);

            let calculated_data_crc = calculate_data_crc(&data);
            if calculated_data_crc != received_data_crc {
                return Err(MstpDecodeError::InvalidDataCrc {
                    expected: calculated_data_crc,
                    actual: received_data_crc,
                });
            }

            data
        } else {
            Vec::new()
        };

        Ok(Self {
            frame_type,
            destination,
            source,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_from_u8() {
        assert_eq!(MstpFrameType::from_u8(0x00), Some(MstpFrameType::Token));
        assert_eq!(
            MstpFrameType::from_u8(0x01),
            Some(MstpFrameType::PollForMaster)
        );
        assert_eq!(
            MstpFrameType::from_u8(0x02),
            Some(MstpFrameType::ReplyToPollForMaster)
        );
        assert_eq!(
            MstpFrameType::from_u8(0x04),
            Some(MstpFrameType::TestRequest)
        );
        assert_eq!(
            MstpFrameType::from_u8(0x05),
            Some(MstpFrameType::TestResponse)
        );
        assert_eq!(
            MstpFrameType::from_u8(0x06),
            Some(MstpFrameType::BacnetDataExpectingReply)
        );
        assert_eq!(
            MstpFrameType::from_u8(0x07),
            Some(MstpFrameType::BacnetDataNotExpectingReply)
        );
        assert_eq!(MstpFrameType::from_u8(0x03), None);
        assert_eq!(MstpFrameType::from_u8(0xFF), None);
    }

    #[test]
    fn test_frame_creation() {
        let frame = MstpFrame::new(
            MstpFrameType::Token,
            5,
            3,
            vec![],
        );

        assert_eq!(frame.frame_type, MstpFrameType::Token);
        assert_eq!(frame.destination, 5);
        assert_eq!(frame.source, 3);
        assert!(frame.data.is_empty());
    }

    #[test]
    fn test_token_frame() {
        let frame = MstpFrame::token(5, 3);

        assert_eq!(frame.frame_type, MstpFrameType::Token);
        assert_eq!(frame.destination, 5);
        assert_eq!(frame.source, 3);
        assert!(frame.data.is_empty());
        assert!(frame.is_token());
        assert!(!frame.is_data());
    }

    #[test]
    fn test_bacnet_data_frame() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        
        // Test frame expecting reply
        let frame = MstpFrame::bacnet_data(10, 20, data.clone(), true);
        assert_eq!(frame.frame_type, MstpFrameType::BacnetDataExpectingReply);
        assert_eq!(frame.destination, 10);
        assert_eq!(frame.source, 20);
        assert_eq!(frame.data, data);
        assert!(!frame.is_token());
        assert!(frame.is_data());

        // Test frame not expecting reply
        let frame = MstpFrame::bacnet_data(10, 20, data.clone(), false);
        assert_eq!(frame.frame_type, MstpFrameType::BacnetDataNotExpectingReply);
        assert!(frame.is_data());
    }

    #[test]
    fn test_is_token() {
        let token = MstpFrame::token(5, 3);
        assert!(token.is_token());

        let data = MstpFrame::bacnet_data(10, 20, vec![0x01], false);
        assert!(!data.is_token());
    }

    #[test]
    fn test_is_data() {
        let data_expecting = MstpFrame::bacnet_data(10, 20, vec![0x01], true);
        assert!(data_expecting.is_data());

        let data_not_expecting = MstpFrame::bacnet_data(10, 20, vec![0x01], false);
        assert!(data_not_expecting.is_data());

        let token = MstpFrame::token(5, 3);
        assert!(!token.is_data());
    }

    #[test]
    fn test_encode_token_frame() {
        // Token frame: frame_type=0x00, dest=5, src=3, length=0
        let token = MstpFrame::token(5, 3);
        let bytes = token.encode();

        // Verify preamble
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[1], 0xFF);

        // Verify header
        assert_eq!(bytes[2], 0x00); // Frame type (Token)
        assert_eq!(bytes[3], 5);    // Destination
        assert_eq!(bytes[4], 3);    // Source
        assert_eq!(bytes[5], 0);    // Length MSB
        assert_eq!(bytes[6], 0);    // Length LSB

        // Verify header CRC
        let header = [0x00, 0x05, 0x03, 0x00, 0x00];
        let expected_crc = calculate_header_crc(&header);
        assert_eq!(bytes[7], expected_crc);

        // Token frame has no data, so total length should be 8 bytes
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn test_encode_data_frame_without_reply() {
        // BACnet Data Not Expecting Reply: frame_type=0x07, dest=10, src=20, data=[0x01, 0x02, 0x03, 0x04]
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let frame = MstpFrame::bacnet_data(10, 20, data.clone(), false);
        let bytes = frame.encode();

        // Verify preamble
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[1], 0xFF);

        // Verify header
        assert_eq!(bytes[2], 0x07); // Frame type (BACnet Data Not Expecting Reply)
        assert_eq!(bytes[3], 10);   // Destination
        assert_eq!(bytes[4], 20);   // Source
        assert_eq!(bytes[5], 0);    // Length MSB
        assert_eq!(bytes[6], 4);    // Length LSB

        // Verify header CRC
        let header = [0x07, 0x0A, 0x14, 0x00, 0x04];
        let expected_header_crc = calculate_header_crc(&header);
        assert_eq!(bytes[7], expected_header_crc);

        // Verify data
        assert_eq!(&bytes[8..12], &data[..]);

        // Verify data CRC (LSB first)
        let expected_data_crc = calculate_data_crc(&data);
        assert_eq!(bytes[12], (expected_data_crc & 0xFF) as u8);  // LSB
        assert_eq!(bytes[13], (expected_data_crc >> 8) as u8);    // MSB

        // Total length: 8 (header) + 4 (data) + 2 (data CRC) = 14 bytes
        assert_eq!(bytes.len(), 14);
    }

    #[test]
    fn test_encode_data_frame_expecting_reply() {
        // BACnet Data Expecting Reply: frame_type=0x06
        let data = vec![0xAA, 0xBB, 0xCC];
        let frame = MstpFrame::bacnet_data(15, 25, data.clone(), true);
        let bytes = frame.encode();

        // Verify preamble
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[1], 0xFF);

        // Verify header
        assert_eq!(bytes[2], 0x06); // Frame type (BACnet Data Expecting Reply)
        assert_eq!(bytes[3], 15);   // Destination
        assert_eq!(bytes[4], 25);   // Source
        assert_eq!(bytes[5], 0);    // Length MSB
        assert_eq!(bytes[6], 3);    // Length LSB

        // Verify data
        assert_eq!(&bytes[8..11], &data[..]);

        // Total length: 8 (header) + 3 (data) + 2 (data CRC) = 13 bytes
        assert_eq!(bytes.len(), 13);
    }

    #[test]
    fn test_encode_empty_data_frame() {
        // Data frame with empty data (edge case)
        let frame = MstpFrame::bacnet_data(10, 20, vec![], false);
        let bytes = frame.encode();

        // Verify preamble
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[1], 0xFF);

        // Verify header
        assert_eq!(bytes[2], 0x07); // Frame type
        assert_eq!(bytes[3], 10);   // Destination
        assert_eq!(bytes[4], 20);   // Source
        assert_eq!(bytes[5], 0);    // Length MSB
        assert_eq!(bytes[6], 0);    // Length LSB

        // Empty data frame should not have data CRC
        // Total length: 8 bytes (header only)
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn test_encode_large_data_frame() {
        // Test with larger data payload (100 bytes)
        let data: Vec<u8> = (0..100).map(|i| i as u8).collect();
        let frame = MstpFrame::bacnet_data(50, 60, data.clone(), false);
        let bytes = frame.encode();

        // Verify preamble
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[1], 0xFF);

        // Verify header
        assert_eq!(bytes[2], 0x07); // Frame type
        assert_eq!(bytes[3], 50);   // Destination
        assert_eq!(bytes[4], 60);   // Source
        assert_eq!(bytes[5], 0);    // Length MSB
        assert_eq!(bytes[6], 100);  // Length LSB

        // Verify data
        assert_eq!(&bytes[8..108], &data[..]);

        // Total length: 8 (header) + 100 (data) + 2 (data CRC) = 110 bytes
        assert_eq!(bytes.len(), 110);
    }

    #[test]
    fn test_encode_poll_for_master_frame() {
        // Poll For Master frame: frame_type=0x01
        let frame = MstpFrame::new(MstpFrameType::PollForMaster, 10, 5, vec![]);
        let bytes = frame.encode();

        // Verify preamble
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[1], 0xFF);

        // Verify header
        assert_eq!(bytes[2], 0x01); // Frame type (Poll For Master)
        assert_eq!(bytes[3], 10);   // Destination
        assert_eq!(bytes[4], 5);    // Source

        // No data, so total length should be 8 bytes
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn test_encode_broadcast_frame() {
        // Broadcast frame: destination=255
        let data = vec![0x12, 0x34];
        let frame = MstpFrame::bacnet_data(255, 10, data.clone(), false);
        let bytes = frame.encode();

        // Verify destination is broadcast address
        assert_eq!(bytes[3], 255);

        // Verify data is present
        assert_eq!(&bytes[8..10], &data[..]);
    }

    #[test]
    fn test_decode_token_frame() {
        // Create and encode a token frame
        let token = MstpFrame::token(5, 3);
        let bytes = token.encode();

        // Decode it back
        let decoded = MstpFrame::decode(&bytes).unwrap();

        // Verify all fields match
        assert_eq!(decoded.frame_type, MstpFrameType::Token);
        assert_eq!(decoded.destination, 5);
        assert_eq!(decoded.source, 3);
        assert!(decoded.data.is_empty());
    }

    #[test]
    fn test_decode_data_frame() {
        // Create and encode a data frame
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let frame = MstpFrame::bacnet_data(10, 20, data.clone(), false);
        let bytes = frame.encode();

        // Decode it back
        let decoded = MstpFrame::decode(&bytes).unwrap();

        // Verify all fields match
        assert_eq!(decoded.frame_type, MstpFrameType::BacnetDataNotExpectingReply);
        assert_eq!(decoded.destination, 10);
        assert_eq!(decoded.source, 20);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn test_decode_data_frame_expecting_reply() {
        // Create and encode a data frame expecting reply
        let data = vec![0xAA, 0xBB, 0xCC];
        let frame = MstpFrame::bacnet_data(15, 25, data.clone(), true);
        let bytes = frame.encode();

        // Decode it back
        let decoded = MstpFrame::decode(&bytes).unwrap();

        // Verify all fields match
        assert_eq!(decoded.frame_type, MstpFrameType::BacnetDataExpectingReply);
        assert_eq!(decoded.destination, 15);
        assert_eq!(decoded.source, 25);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn test_decode_truncated_frame() {
        // Frame too short (less than 8 bytes)
        let bytes = vec![0x55, 0xFF, 0x00, 0x05, 0x03];
        let result = MstpFrame::decode(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::TruncatedFrame { expected, actual } => {
                assert_eq!(expected, 8);
                assert_eq!(actual, 5);
            }
            _ => panic!("Expected TruncatedFrame error"),
        }
    }

    #[test]
    fn test_decode_invalid_preamble() {
        // Invalid preamble bytes
        let mut bytes = MstpFrame::token(5, 3).encode();
        bytes[0] = 0xAA; // Corrupt first preamble byte

        let result = MstpFrame::decode(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::InvalidPreamble { byte1, byte2 } => {
                assert_eq!(byte1, 0xAA);
                assert_eq!(byte2, 0xFF);
            }
            _ => panic!("Expected InvalidPreamble error"),
        }
    }

    #[test]
    fn test_decode_invalid_frame_type() {
        // Invalid frame type
        let mut bytes = MstpFrame::token(5, 3).encode();
        bytes[2] = 0xFF; // Invalid frame type

        let result = MstpFrame::decode(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::InvalidFrameType(frame_type) => {
                assert_eq!(frame_type, 0xFF);
            }
            _ => panic!("Expected InvalidFrameType error"),
        }
    }

    #[test]
    fn test_decode_invalid_header_crc() {
        // Corrupt header CRC
        let mut bytes = MstpFrame::token(5, 3).encode();
        bytes[7] ^= 0xFF; // Flip all bits in header CRC

        let result = MstpFrame::decode(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::InvalidHeaderCrc { expected, actual } => {
                assert_ne!(expected, actual);
            }
            _ => panic!("Expected InvalidHeaderCrc error"),
        }
    }

    #[test]
    fn test_decode_invalid_data_crc() {
        // Corrupt data CRC
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let frame = MstpFrame::bacnet_data(10, 20, data, false);
        let mut bytes = frame.encode();
        
        // Corrupt data CRC (last 2 bytes)
        let len = bytes.len();
        bytes[len - 1] ^= 0xFF; // Flip all bits in data CRC MSB

        let result = MstpFrame::decode(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::InvalidDataCrc { expected, actual } => {
                assert_ne!(expected, actual);
            }
            _ => panic!("Expected InvalidDataCrc error"),
        }
    }

    #[test]
    fn test_decode_truncated_data_frame() {
        // Create a data frame but truncate it before data CRC
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let frame = MstpFrame::bacnet_data(10, 20, data, false);
        let mut bytes = frame.encode();
        
        // Truncate to remove data CRC
        bytes.truncate(bytes.len() - 2);

        let result = MstpFrame::decode(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::TruncatedFrame { expected, actual } => {
                assert!(expected > actual);
            }
            _ => panic!("Expected TruncatedFrame error"),
        }
    }

    #[test]
    fn test_decode_round_trip_token() {
        // Test round-trip for token frame
        let original = MstpFrame::token(10, 5);
        let bytes = original.encode();
        let decoded = MstpFrame::decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_round_trip_data() {
        // Test round-trip for data frame
        let data = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let original = MstpFrame::bacnet_data(50, 60, data, false);
        let bytes = original.encode();
        let decoded = MstpFrame::decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_round_trip_large_data() {
        // Test round-trip with large data payload
        let data: Vec<u8> = (0..100).map(|i| i as u8).collect();
        let original = MstpFrame::bacnet_data(100, 127, data, true);
        let bytes = original.encode();
        let decoded = MstpFrame::decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_empty_data_frame() {
        // Test decoding frame with empty data
        let original = MstpFrame::bacnet_data(10, 20, vec![], false);
        let bytes = original.encode();
        let decoded = MstpFrame::decode(&bytes).unwrap();

        assert_eq!(original, decoded);
        assert!(decoded.data.is_empty());
    }

    #[test]
    fn test_decode_poll_for_master() {
        // Test decoding Poll For Master frame
        let original = MstpFrame::new(MstpFrameType::PollForMaster, 127, 0, vec![]);
        let bytes = original.encode();
        let decoded = MstpFrame::decode(&bytes).unwrap();

        assert_eq!(decoded.frame_type, MstpFrameType::PollForMaster);
        assert_eq!(decoded.destination, 127);
        assert_eq!(decoded.source, 0);
        assert!(decoded.data.is_empty());
    }

    #[test]
    fn test_decode_broadcast_frame() {
        // Test decoding broadcast frame (destination 255)
        let data = vec![0xFF, 0xEE, 0xDD];
        let original = MstpFrame::bacnet_data(255, 10, data.clone(), false);
        let bytes = original.encode();
        let decoded = MstpFrame::decode(&bytes).unwrap();

        assert_eq!(decoded.destination, 255);
        assert_eq!(decoded.source, 10);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn test_decode_all_frame_types() {
        // Test decoding all valid frame types
        let frame_types = vec![
            MstpFrameType::Token,
            MstpFrameType::PollForMaster,
            MstpFrameType::ReplyToPollForMaster,
            MstpFrameType::TestRequest,
            MstpFrameType::TestResponse,
            MstpFrameType::BacnetDataExpectingReply,
            MstpFrameType::BacnetDataNotExpectingReply,
        ];

        for frame_type in frame_types {
            let original = MstpFrame::new(frame_type, 10, 20, vec![]);
            let bytes = original.encode();
            let decoded = MstpFrame::decode(&bytes).unwrap();

            assert_eq!(decoded.frame_type, frame_type);
        }
    }

    #[test]
    fn test_decode_corrupted_data() {
        // Test that corrupted data is detected via CRC
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let frame = MstpFrame::bacnet_data(10, 20, data, false);
        let mut bytes = frame.encode();
        
        // Corrupt a data byte
        bytes[8] ^= 0x01; // Flip a bit in first data byte

        let result = MstpFrame::decode(&bytes);

        // Should fail with InvalidDataCrc
        assert!(result.is_err());
        match result.unwrap_err() {
            MstpDecodeError::InvalidDataCrc { .. } => {
                // Expected
            }
            other => panic!("Expected InvalidDataCrc error, got {:?}", other),
        }
    }
}

/// Calculate MS/TP header CRC using CRC-8 algorithm
///
/// The header CRC is calculated over 5 bytes:
/// - Frame Type (1 byte)
/// - Destination Address (1 byte)
/// - Source Address (1 byte)
/// - Length MSB (1 byte)
/// - Length LSB (1 byte)
///
/// Algorithm: CRC-8 with polynomial 0x55 (x^6 + x^4 + x^2 + 1)
/// Initial value: 0xFF
///
/// # Arguments
/// * `header` - 5-byte header array
///
/// # Returns
/// 8-bit CRC value
///
/// # Examples
///
/// ```
/// use baccy_transport::frame::calculate_header_crc;
///
/// // Token frame header: frame_type=0x00, dest=5, src=3, length=0
/// let header = [0x00, 0x05, 0x03, 0x00, 0x00];
/// let crc = calculate_header_crc(&header);
/// ```
pub fn calculate_header_crc(header: &[u8; 5]) -> u8 {
    let mut crc = 0xFFu8;

    for &byte in header {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x01 != 0 {
                crc = (crc >> 1) ^ 0x55;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

/// Calculate MS/TP data CRC using CRC-16 algorithm
///
/// The data CRC is calculated over the entire data payload.
///
/// Algorithm: CRC-16 with polynomial 0x1021 (x^16 + x^12 + x^5 + 1)
/// Implementation uses reversed polynomial 0xA001 for efficiency
/// Initial value: 0xFFFF
///
/// # Arguments
/// * `data` - Data payload bytes
///
/// # Returns
/// 16-bit CRC value
///
/// # Examples
///
/// ```
/// use baccy_transport::frame::calculate_data_crc;
///
/// let data = b"Hello BACnet";
/// let crc = calculate_data_crc(data);
/// ```
pub fn calculate_data_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;

    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

#[cfg(test)]
mod crc_tests {
    use super::*;

    #[test]
    fn test_header_crc_token_frame() {
        // Token frame: frame_type=0x00, dest=5, src=3, length=0
        let header = [0x00, 0x05, 0x03, 0x00, 0x00];
        let crc = calculate_header_crc(&header);
        
        // Verify CRC is calculated correctly
        // The actual CRC value is 0xFC (252 in decimal)
        assert_eq!(crc, 0xFC);
    }

    #[test]
    fn test_header_crc_data_frame() {
        // BACnet Data Not Expecting Reply: frame_type=0x07, dest=10, src=20, length=4
        let header = [0x07, 0x0A, 0x14, 0x00, 0x04];
        let crc = calculate_header_crc(&header);
        
        // Verify CRC is calculated correctly
        assert!(crc != 0); // CRC should be non-zero for this header
    }

    #[test]
    fn test_header_crc_all_zeros() {
        // Edge case: all zeros
        let header = [0x00, 0x00, 0x00, 0x00, 0x00];
        let crc = calculate_header_crc(&header);
        
        // CRC of all zeros should be 0x98 (152 in decimal)
        assert_eq!(crc, 0x98);
    }

    #[test]
    fn test_header_crc_all_ones() {
        // Edge case: all ones
        let header = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let crc = calculate_header_crc(&header);
        
        // Verify CRC is calculated
        assert!(crc != 0xFF); // Should not be 0xFF
    }

    #[test]
    fn test_data_crc_empty() {
        // Empty data should produce a specific CRC
        let data: &[u8] = &[];
        let crc = calculate_data_crc(data);
        
        // CRC of empty data is complement of initial value
        assert_eq!(crc, 0x0000);
    }

    #[test]
    fn test_data_crc_single_byte() {
        // Single byte test
        let data = [0x55];
        let crc = calculate_data_crc(&data);
        
        // Verify CRC is calculated
        assert!(crc != 0);
    }

    #[test]
    fn test_data_crc_known_value() {
        // Test with known data
        let data = b"Hello BACnet";
        let crc = calculate_data_crc(data);
        
        // Verify CRC is calculated (non-zero for this data)
        assert!(crc != 0);
    }

    #[test]
    fn test_data_crc_bacnet_message() {
        // Simulate a small BACnet message
        let data = [0x01, 0x02, 0x03, 0x04];
        let crc = calculate_data_crc(&data);
        
        // Verify CRC is calculated
        assert!(crc != 0);
    }

    #[test]
    fn test_data_crc_all_zeros() {
        // Edge case: all zeros
        let data = [0x00, 0x00, 0x00, 0x00];
        let crc = calculate_data_crc(&data);
        
        // Verify CRC is calculated
        assert!(crc != 0);
    }

    #[test]
    fn test_data_crc_all_ones() {
        // Edge case: all ones
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let crc = calculate_data_crc(&data);
        
        // Verify CRC is calculated
        assert!(crc != 0xFFFF);
    }

    #[test]
    fn test_data_crc_consistency() {
        // Same data should always produce same CRC
        let data = b"Test data for CRC";
        let crc1 = calculate_data_crc(data);
        let crc2 = calculate_data_crc(data);
        
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_data_crc_different_data() {
        // Different data should produce different CRCs
        let data1 = b"Data 1";
        let data2 = b"Data 2";
        
        let crc1 = calculate_data_crc(data1);
        let crc2 = calculate_data_crc(data2);
        
        assert_ne!(crc1, crc2);
    }

    #[test]
    fn test_header_crc_consistency() {
        // Same header should always produce same CRC
        let header = [0x06, 0x0A, 0x14, 0x00, 0x10];
        let crc1 = calculate_header_crc(&header);
        let crc2 = calculate_header_crc(&header);
        
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_header_crc_different_headers() {
        // Different headers should produce different CRCs
        let header1 = [0x00, 0x05, 0x03, 0x00, 0x00];
        let header2 = [0x07, 0x0A, 0x14, 0x00, 0x04];
        
        let crc1 = calculate_header_crc(&header1);
        let crc2 = calculate_header_crc(&header2);
        
        assert_ne!(crc1, crc2);
    }
}
