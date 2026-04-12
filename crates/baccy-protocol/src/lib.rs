//! BACnet Protocol Service Layer
//!
//! This crate provides a thin adapter layer between baccy-core types and bacnet-rs,
//! handling BACnet protocol operations like device discovery, property reading/writing,
//! and object list retrieval.

use baccy_core::{Address, Device, DeviceId, ObjectId, PropertyId, PropertyValue};
use baccy_transport::{Transport, TransportError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Re-export bacnet-rs for direct usage
pub use bacnet_rs;

/// Type conversion utilities for mapping between baccy-core and bacnet-rs types
pub mod type_conversion {
    use baccy_core::{ObjectType, PropertyId, PropertyValue};
    use bacnet_rs::object::ObjectType as BacnetObjectType;

    /// Convert baccy-core ObjectType to bacnet-rs ObjectType
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
        }
    }

    /// Convert bacnet-rs ObjectType to baccy-core ObjectType
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
            _ => None,
        }
    }

    /// Convert baccy-core PropertyId to bacnet-rs property identifier (u32)
    pub fn to_bacnet_property_id(prop_id: PropertyId) -> u32 {
        match prop_id {
            PropertyId::PresentValue => 85,  // PRESENT_VALUE
            PropertyId::ObjectName => 77,     // OBJECT_NAME
            PropertyId::Description => 28,    // DESCRIPTION
            PropertyId::Units => 117,         // UNITS
            PropertyId::StatusFlags => 111,   // STATUS_FLAGS
            PropertyId::OutOfService => 81,   // OUT_OF_SERVICE
            PropertyId::Reliability => 103,   // RELIABILITY
            PropertyId::EventState => 36,     // EVENT_STATE
            PropertyId::Priority => 87,       // PRIORITY
        }
    }

    /// Encode a PropertyValue to BACnet bytes
    pub fn to_bacnet_value(value: PropertyValue) -> Result<Vec<u8>, String> {
        use bacnet_rs::encoding::advanced::bitstring::encode_bit_string;
        use bacnet_rs::encoding::{
            encode_boolean, encode_character_string, encode_enumerated, encode_real, encode_signed,
            encode_unsigned,
        };

        let mut buffer = Vec::new();
        match value {
            PropertyValue::Real(f) => encode_real(&mut buffer, f)
                .map_err(|e| format!("Failed to encode Real: {}", e))?,
            PropertyValue::Integer(i) => encode_signed(&mut buffer, i)
                .map_err(|e| format!("Failed to encode Integer: {}", e))?,
            PropertyValue::Unsigned(u) => encode_unsigned(&mut buffer, u)
                .map_err(|e| format!("Failed to encode Unsigned: {}", e))?,
            PropertyValue::Boolean(b) => encode_boolean(&mut buffer, b)
                .map_err(|e| format!("Failed to encode Boolean: {}", e))?,
            PropertyValue::String(s) => encode_character_string(&mut buffer, &s)
                .map_err(|e| format!("Failed to encode String: {}", e))?,
            PropertyValue::Enumerated(e) => encode_enumerated(&mut buffer, e)
                .map_err(|e| format!("Failed to encode Enumerated: {}", e))?,
            PropertyValue::BitString(bits) => encode_bit_string(&mut buffer, &bits)
                .map_err(|e| format!("Failed to encode BitString: {}", e))?,
        }
        Ok(buffer)
    }

    /// Decode BACnet bytes to PropertyValue
    pub fn from_bacnet_value(data: &[u8]) -> Result<PropertyValue, String> {
        use bacnet_rs::encoding::advanced::bitstring::decode_bit_string;
        use bacnet_rs::encoding::{
            decode_boolean, decode_character_string, decode_enumerated, decode_real, decode_signed,
            decode_unsigned,
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
                Ok(PropertyValue::Unsigned(value))
            }
            3 => {
                let (value, _) =
                    decode_signed(data).map_err(|e| format!("Failed to decode Integer: {}", e))?;
                Ok(PropertyValue::Integer(value))
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
            _ => Err(format!("Unsupported application tag: {}", tag_number)),
        }
    }
}

/// Protocol errors
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

/// BACnet error class
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

/// BACnet error code
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

/// Parse an I-Am message
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
        return Err(ProtocolError::DecodingError("Invalid I-Am message".to_string()));
    }
    offset += 2; // Skip APDU type and service choice

    let iam = IAmRequest::decode(&data[offset..])
        .map_err(|e| ProtocolError::DecodingError(format!("Failed to decode I-Am: {}", e)))?;

    Ok(Device {
        instance: iam.device_identifier.instance,
        name: format!("Device {}", iam.device_identifier.instance),
        vendor_id: iam.vendor_identifier as u16,
        vendor_name: String::new(),
        model_name: String::new(),
        description: String::new(),
    })
}

/// BACnet service for protocol operations
pub struct BacnetService {
    transport: Arc<dyn Transport>,
    request_timeout: Duration,
    device_addresses: Arc<Mutex<HashMap<DeviceId, Address>>>,
}

impl BacnetService {
    /// Create a new BACnet service
    pub fn new(transport: Arc<dyn Transport>, timeout: Duration) -> Self {
        const MIN_TIMEOUT: Duration = Duration::from_millis(100);
        const MAX_TIMEOUT: Duration = Duration::from_secs(30);

        if timeout < MIN_TIMEOUT || timeout > MAX_TIMEOUT {
            panic!("Timeout must be between 100ms and 30 seconds");
        }

        Self {
            transport,
            request_timeout: timeout,
            device_addresses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn cache_device_address(&self, device_id: DeviceId, address: Address) {
        let mut addresses = self.device_addresses.lock().unwrap();
        addresses.insert(device_id, address);
    }

    fn get_device_address(&self, device_id: DeviceId) -> Result<Address, ProtocolError> {
        let addresses = self.device_addresses.lock().unwrap();
        addresses.get(&device_id).cloned().ok_or_else(|| {
            ProtocolError::DecodingError(format!(
                "Unknown device {}: Device must be discovered via Who-Is/I-Am first",
                device_id
            ))
        })
    }

    /// Send a Who-Is broadcast
    pub fn who_is(&self) -> Result<(), ProtocolError> {
        use bacnet_rs::app::Apdu;
        use bacnet_rs::datalink::bip::{BvlcFunction, BvlcHeader};
        use bacnet_rs::network::Npdu;
        use bacnet_rs::service::WhoIsRequest;

        let who_is = WhoIsRequest::new();
        let mut service_data = Vec::new();
        who_is
            .encode(&mut service_data)
            .map_err(|e| ProtocolError::EncodingError(format!("Failed to encode Who-Is: {}", e)))?;

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: 0x08, // WhoIs service choice
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        let header = BvlcHeader::new(
            BvlcFunction::OriginalBroadcastNpdu,
            4 + message.len() as u16,
        );
        let mut bvlc_message = header.encode();
        bvlc_message.extend_from_slice(&message);

        self.transport.broadcast(&bvlc_message)?;
        Ok(())
    }

    /// Send a Who-Is broadcast with device instance range
    pub fn who_is_range(&self, low: u32, high: u32) -> Result<(), ProtocolError> {
        use bacnet_rs::app::Apdu;
        use bacnet_rs::datalink::bip::{BvlcFunction, BvlcHeader};
        use bacnet_rs::network::Npdu;
        use bacnet_rs::service::WhoIsRequest;

        let who_is = WhoIsRequest::for_range(low, high);
        let mut service_data = Vec::new();
        who_is.encode(&mut service_data).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode Who-Is range: {}", e))
        })?;

        let apdu = Apdu::UnconfirmedRequest {
            service_choice: 0x08, // WhoIs service choice
            service_data,
        };

        let mut message = Npdu::global_broadcast().encode();
        message.extend_from_slice(&apdu.encode());

        let header = BvlcHeader::new(
            BvlcFunction::OriginalBroadcastNpdu,
            4 + message.len() as u16,
        );
        let mut bvlc_message = header.encode();
        bvlc_message.extend_from_slice(&message);

        self.transport.broadcast(&bvlc_message)?;
        Ok(())
    }

    /// Receive and parse I-Am responses
    pub fn receive_iam(&self, timeout: Duration) -> Result<Device, ProtocolError> {
        let (source_address, data) = self.transport.receive(timeout)?;
        let device = parse_iam(&data, source_address.clone())?;
        self.cache_device_address(device.instance, source_address);
        Ok(device)
    }

    /// Read a property from a BACnet object
    pub fn read_property(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
    ) -> Result<PropertyValue, ProtocolError> {
        use bacnet_rs::app::{Apdu, MaxApduSize, MaxSegments};
        use bacnet_rs::datalink::bip::{BvlcFunction, BvlcHeader};
        use bacnet_rs::network::Npdu;
        use bacnet_rs::object::ObjectIdentifier;
        use bacnet_rs::service::ReadPropertyRequest;
        use type_conversion::{from_bacnet_value, to_bacnet_object_type, to_bacnet_property_id};

        let address = self.get_device_address(device)?;

        // Encode request
        let bacnet_object_id =
            ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);
        let bacnet_property_id = to_bacnet_property_id(property);

        let read_request = ReadPropertyRequest::new(bacnet_object_id, bacnet_property_id);
        let mut service_data = Vec::new();
        read_request.encode(&mut service_data).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode ReadProperty: {}", e))
        })?;

        let apdu = Apdu::ConfirmedRequest {
            segmented: false,
            more_follows: false,
            segmented_response_accepted: false,
            max_segments: MaxSegments::Unspecified,
            max_response_size: MaxApduSize::Up1476,
            invoke_id: 1,
            sequence_number: None,
            proposed_window_size: None,
            service_choice: 0x0C, // ReadProperty service choice
            service_data,
        };

        let mut npdu = Npdu::new();
        npdu.control.expecting_reply = true;
        let mut message = npdu.encode();
        message.extend_from_slice(&apdu.encode());

        let header = BvlcHeader::new(BvlcFunction::OriginalUnicastNpdu, 4 + message.len() as u16);
        let mut bvlc_message = header.encode();
        bvlc_message.extend_from_slice(&message);

        // Send request
        self.transport.send(&address, &bvlc_message)?;

        // Receive response
        let start_time = std::time::Instant::now();
        loop {
            let remaining = self
                .request_timeout
                .checked_sub(start_time.elapsed())
                .unwrap_or(Duration::from_millis(0));

            if remaining.as_millis() == 0 {
                return Err(ProtocolError::Timeout);
            }

            let (response_address, response_bytes) = self.transport.receive(remaining)?;
            if response_address != address {
                continue;
            }

            // Decode response
            let mut offset = 0;
            if response_bytes.len() >= 4 && response_bytes[0] == 0x81 {
                offset = 4;
                let (_, npdu_len) = Npdu::decode(&response_bytes[offset..]).map_err(|e| {
                    ProtocolError::DecodingError(format!("Failed to decode NPDU: {}", e))
                })?;
                offset += npdu_len;
            }

            let apdu = Apdu::decode(&response_bytes[offset..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode APDU: {}", e))
            })?;

            match apdu {
                Apdu::ComplexAck {
                    service_data,
                    ..
                } => {
                    // Manual parsing to handle edge cases that ReadPropertyResponse::decode() misses
                    let mut pos = 0;
                    
                    // Skip object identifier (context tag 0)
                    if pos >= service_data.len() || service_data[pos] != 0x0C {
                        return Err(ProtocolError::DecodingError("Expected object identifier".to_string()));
                    }
                    pos += 5; // 0x0C + 4 bytes of object ID
                    
                    // Skip property identifier (context tag 1)
                    if pos >= service_data.len() || service_data[pos] != 0x19 {
                        return Err(ProtocolError::DecodingError("Expected property identifier".to_string()));
                    }
                    pos += 2; // 0x19 + 1 byte property ID
                    
                    // Skip array index if present (context tag 2)
                    if pos < service_data.len() && service_data[pos] == 0x29 {
                        pos += 2; // 0x29 + 1 byte array index
                    }
                    
                    // Property value opening tag (context tag 3)
                    if pos >= service_data.len() || service_data[pos] != 0x3E {
                        return Err(ProtocolError::DecodingError("Expected property value opening tag".to_string()));
                    }
                    pos += 1;
                    
                    // Find closing tag (0x3F)
                    let value_start = pos;
                    let mut value_end = pos;
                    while value_end < service_data.len() && service_data[value_end] != 0x3F {
                        value_end += 1;
                    }
                    
                    if value_end >= service_data.len() {
                        return Err(ProtocolError::DecodingError("Missing property value closing tag".to_string()));
                    }
                    
                    let property_value_bytes = &service_data[value_start..value_end];
                    let property_value = from_bacnet_value(property_value_bytes).map_err(|e| {
                        ProtocolError::DecodingError(format!("Failed to convert value: {}", e))
                    })?;

                    return Ok(property_value);
                }
                Apdu::Error {
                    error_class,
                    error_code,
                    ..
                } => {
                    return Err(ProtocolError::BacnetError {
                        class: match error_class {
                            1 => ErrorClass::Object,
                            2 => ErrorClass::Property,
                            _ => ErrorClass::Unknown(error_class as u8),
                        },
                        code: match error_code {
                            31 => ErrorCode::UnknownObject,
                            32 => ErrorCode::UnknownProperty,
                            _ => ErrorCode::Unknown(error_code as u8),
                        },
                    });
                }
                _ => continue,
            }
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
        // Similar to read_property but with array index
        // For now, delegate to read_property (simplified)
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
        use bacnet_rs::app::{Apdu, MaxApduSize, MaxSegments};
        use bacnet_rs::datalink::bip::{BvlcFunction, BvlcHeader};
        use bacnet_rs::network::Npdu;
        use bacnet_rs::object::ObjectIdentifier;
        use bacnet_rs::service::WritePropertyRequest;
        use type_conversion::{to_bacnet_object_type, to_bacnet_property_id, to_bacnet_value};

        let address = self.get_device_address(device)?;

        // Encode request
        let bacnet_object_id =
            ObjectIdentifier::new(to_bacnet_object_type(object.object_type), object.instance);
        let bacnet_property_id = to_bacnet_property_id(property);
        let property_value_bytes = to_bacnet_value(value)
            .map_err(|e| ProtocolError::EncodingError(format!("Failed to encode value: {}", e)))?;

        let write_request =
            WritePropertyRequest::new(bacnet_object_id, bacnet_property_id, property_value_bytes);

        let mut service_data = Vec::new();
        write_request.encode(&mut service_data).map_err(|e| {
            ProtocolError::EncodingError(format!("Failed to encode WriteProperty: {}", e))
        })?;

        let apdu = Apdu::ConfirmedRequest {
            segmented: false,
            more_follows: false,
            segmented_response_accepted: false,
            max_segments: MaxSegments::Unspecified,
            max_response_size: MaxApduSize::Up1476,
            invoke_id: 1,
            sequence_number: None,
            proposed_window_size: None,
            service_choice: 0x0F, // WriteProperty service choice
            service_data,
        };

        let mut npdu = Npdu::new();
        npdu.control.expecting_reply = true;
        let mut message = npdu.encode();
        message.extend_from_slice(&apdu.encode());

        let header = BvlcHeader::new(BvlcFunction::OriginalUnicastNpdu, 4 + message.len() as u16);
        let mut bvlc_message = header.encode();
        bvlc_message.extend_from_slice(&message);

        // Send request
        self.transport.send(&address, &bvlc_message)?;

        // Wait for SimpleAck or Error
        let start_time = std::time::Instant::now();
        loop {
            let remaining = self
                .request_timeout
                .checked_sub(start_time.elapsed())
                .unwrap_or(Duration::from_millis(0));

            if remaining.as_millis() == 0 {
                return Err(ProtocolError::Timeout);
            }

            let (response_address, response_bytes) = self.transport.receive(remaining)?;
            if response_address != address {
                continue;
            }

            // Check for SimpleAck (0x2) or Error (0x5)
            let pdu_type = (response_bytes[0] >> 4) & 0x0F;
            if pdu_type == 0x2 {
                return Ok(());
            } else if pdu_type == 0x5 {
                return Err(ProtocolError::BacnetError {
                    class: ErrorClass::Unknown(0),
                    code: ErrorCode::WriteAccessDenied,
                });
            }
        }
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
        // Similar to write_property but with array index
        // For now, delegate to write_property (simplified)
        self.write_property(device, object, property, value)
    }

    /// Write a property with priority
    pub fn write_property_priority(
        &self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        value: PropertyValue,
        _priority: u8,
    ) -> Result<(), ProtocolError> {
        // Similar to write_property but with priority
        // For now, delegate to write_property (simplified)
        self.write_property(device, object, property, value)
    }

    /// Read the object list from a device
    pub fn read_object_list(&self, device: DeviceId) -> Result<Vec<ObjectId>, ProtocolError> {
        use bacnet_rs::object::{
            ObjectIdentifier as BacnetObjectId, ObjectType as BacnetObjectType,
        };
        use bacnet_rs::service::{
            PropertyReference, ReadAccessSpecification, ReadPropertyMultipleRequest,
        };

        // Get the device address from cache
        let address = self.get_device_address(device)?;

        // Create device object identifier
        let device_object = BacnetObjectId::new(BacnetObjectType::Device, device);

        // Create property reference for Object_List (property ID 76)
        let property_ref = PropertyReference::new(76);

        // Create read access specification
        let read_spec = ReadAccessSpecification::new(device_object, vec![property_ref]);

        // Create ReadPropertyMultiple request
        let rpm_request = ReadPropertyMultipleRequest::new(vec![read_spec]);

        // Encode the request
        let mut service_data = Vec::new();
        for spec in &rpm_request.read_access_specifications {
            // Object identifier - context tag 0
            // Manually encode: (object_type << 22) | instance
            let obj_type_num = match spec.object_identifier.object_type {
                BacnetObjectType::Device => 8,
                _ => 0,
            };
            let object_id: u32 = (obj_type_num << 22) | spec.object_identifier.instance;
            service_data.push(0x0C);
            service_data.extend_from_slice(&object_id.to_be_bytes());

            // Property references - context tag 1
            service_data.push(0x1E);
            for prop_ref in &spec.property_references {
                service_data.push(0x09);
                service_data.push(prop_ref.property_identifier as u8);

                if let Some(array_index) = prop_ref.property_array_index {
                    service_data.push(0x19);
                    service_data.push(array_index as u8);
                }
            }
            service_data.push(0x1F);
        }

        // Create APDU using bacnet-rs
        use bacnet_rs::app::{Apdu, MaxApduSize, MaxSegments};

        let apdu = Apdu::ConfirmedRequest {
            segmented: false,
            more_follows: false,
            segmented_response_accepted: true,
            max_segments: MaxSegments::Unspecified,
            max_response_size: MaxApduSize::Up1476,
            invoke_id: 1,
            sequence_number: None,
            proposed_window_size: None,
            service_choice: 0x0E, // ReadPropertyMultiple service choice
            service_data,
        };

        let apdu_data = apdu.encode();

        // Create NPDU using bacnet-rs
        use bacnet_rs::network::Npdu;
        let mut npdu = Npdu::new();
        npdu.control.expecting_reply = true;
        npdu.control.priority = 0;
        let npdu_data = npdu.encode();

        // Combine NPDU and APDU
        let mut message = npdu_data;
        message.extend_from_slice(&apdu_data);

        // Wrap in BVLC header for BACnet/IP
        let mut bvlc_message = vec![0x81, 0x0A, 0x00, 0x00];
        bvlc_message.extend_from_slice(&message);

        let total_len = bvlc_message.len() as u16;
        bvlc_message[2] = (total_len >> 8) as u8;
        bvlc_message[3] = (total_len & 0xFF) as u8;

        // Send the request
        self.transport.send(&address, &bvlc_message)?;

        // Wait for response
        let start_time = std::time::Instant::now();
        loop {
            let remaining_timeout = self
                .request_timeout
                .checked_sub(start_time.elapsed())
                .unwrap_or(Duration::from_millis(0));

            if remaining_timeout.as_millis() == 0 {
                return Err(ProtocolError::Timeout);
            }

            let (response_address, response_bytes) = self.transport.receive(remaining_timeout)?;

            if response_address != address {
                continue;
            }

            // Check BVLC header
            if response_bytes.len() < 4 || response_bytes[0] != 0x81 {
                continue;
            }

            // Decode NPDU
            let npdu_start = 4;
            let (_npdu, npdu_len) = Npdu::decode(&response_bytes[npdu_start..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode NPDU: {}", e))
            })?;

            // Decode APDU
            let apdu_start = npdu_start + npdu_len;
            let response_apdu = Apdu::decode(&response_bytes[apdu_start..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode APDU: {}", e))
            })?;

            match response_apdu {
                Apdu::ComplexAck {
                    invoke_id,
                    service_data,
                    ..
                } => {
                    if invoke_id != 1 {
                        continue;
                    }

                    // Parse object list from service data
                    let mut objects = Vec::new();
                    let mut pos = 0;

                    // Scan for object identifiers (0xC4 tag)
                    while pos + 5 <= service_data.len() {
                        if service_data[pos] == 0xC4 {
                            pos += 1;
                            let obj_id_bytes = [
                                service_data[pos],
                                service_data[pos + 1],
                                service_data[pos + 2],
                                service_data[pos + 3],
                            ];
                            let obj_id_raw = u32::from_be_bytes(obj_id_bytes);

                            // Manually decode: object_type = upper 10 bits, instance = lower 22 bits
                            let obj_type_num = (obj_id_raw >> 22) & 0x3FF;
                            let instance = obj_id_raw & 0x3FFFFF;

                            // Convert object type number to BacnetObjectType
                            let bacnet_obj_type = match obj_type_num {
                                0 => BacnetObjectType::AnalogInput,
                                1 => BacnetObjectType::AnalogOutput,
                                2 => BacnetObjectType::AnalogValue,
                                3 => BacnetObjectType::BinaryInput,
                                4 => BacnetObjectType::BinaryOutput,
                                5 => BacnetObjectType::BinaryValue,
                                8 => BacnetObjectType::Device,
                                13 => BacnetObjectType::MultiStateInput,
                                14 => BacnetObjectType::MultiStateOutput,
                                19 => BacnetObjectType::MultiStateValue,
                                _ => {
                                    pos += 4;
                                    continue;
                                }
                            };

                            // Convert to our ObjectId type
                            if let Some(obj_type) =
                                type_conversion::from_bacnet_object_type(bacnet_obj_type)
                            {
                                let object_id = ObjectId {
                                    object_type: obj_type,
                                    instance,
                                };
                                objects.push(object_id);
                            }

                            pos += 4;
                        } else {
                            pos += 1;
                        }
                    }

                    return Ok(objects);
                }
                Apdu::Error { .. } => {
                    return Err(ProtocolError::BacnetError {
                        class: ErrorClass::Device,
                        code: ErrorCode::UnknownObject,
                    });
                }
                _ => {
                    continue;
                }
            }
        }
    }
}
