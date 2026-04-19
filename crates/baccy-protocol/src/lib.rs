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

// Import service choice enums from bacnet-rs
use bacnet_rs::service::{ConfirmedServiceChoice, UnconfirmedServiceChoice};
use bacnet_rs::object::property_identifier::PropertyIdentifier;

/// Type conversion utilities for mapping between baccy-core and bacnet-rs types
pub mod type_conversion {
    use baccy_core::{ObjectType, PropertyId, PropertyValue};
    use bacnet_rs::object::ObjectType as BacnetObjectType;
    use bacnet_rs::object::property_identifier::PropertyIdentifier;

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

    /// Convert baccy-core PropertyId to bacnet-rs PropertyIdentifier enum
    pub fn to_bacnet_property_id(prop_id: PropertyId) -> PropertyIdentifier {
        match prop_id {
            PropertyId::PresentValue => PropertyIdentifier::PresentValue,
            PropertyId::ObjectName => PropertyIdentifier::ObjectName,
            PropertyId::Description => PropertyIdentifier::Description,
            PropertyId::Units => PropertyIdentifier::Units,
            PropertyId::StatusFlags => PropertyIdentifier::StatusFlags,
            PropertyId::OutOfService => PropertyIdentifier::OutOfService,
            PropertyId::Reliability => PropertyIdentifier::Reliability,
            PropertyId::EventState => PropertyIdentifier::EventState,
            PropertyId::Priority => PropertyIdentifier::Priority,
        }
    }

    /// Encode a PropertyValue to BACnet bytes
    pub fn to_bacnet_value(value: PropertyValue) -> Result<Vec<u8>, String> {
        use bacnet_rs::encoding::advanced::bitstring::encode_bit_string;
        use bacnet_rs::encoding::{
            encode_boolean, encode_character_string, encode_enumerated, encode_real, encode_signed,
            encode_unsigned, encode_object_identifier,
        };

        let mut buffer = Vec::new();
        match value {
            PropertyValue::Real(f) => encode_real(&mut buffer, f)
                .map_err(|e| format!("Failed to encode Real: {}", e))?,
            PropertyValue::Integer(i) => encode_signed(&mut buffer, i as i32)
                .map_err(|e| format!("Failed to encode Integer: {}", e))?,
            PropertyValue::Unsigned(u) => encode_unsigned(&mut buffer, u as u32)
                .map_err(|e| format!("Failed to encode Unsigned: {}", e))?,
            PropertyValue::Boolean(b) => encode_boolean(&mut buffer, b)
                .map_err(|e| format!("Failed to encode Boolean: {}", e))?,
            PropertyValue::String(s) => encode_character_string(&mut buffer, &s)
                .map_err(|e| format!("Failed to encode String: {}", e))?,
            PropertyValue::Enumerated(e) => {
                // encode_enumerated is now infallible in bacnet-rs 0.3
                encode_enumerated(&mut buffer, e);
            },
            PropertyValue::BitString(bits) => encode_bit_string(&mut buffer, &bits)
                .map_err(|e| format!("Failed to encode BitString: {}", e))?,
            PropertyValue::ObjectIdentifier { object_type, instance } => {
                let bacnet_obj_id = bacnet_rs::object::ObjectIdentifier::new(
                    to_bacnet_object_type(object_type),
                    instance
                );
                encode_object_identifier(&mut buffer, bacnet_obj_id)
                    .map_err(|e| format!("Failed to encode ObjectIdentifier: {}", e))?;
            },
        }
        Ok(buffer)
    }

    /// Decode BACnet bytes to PropertyValue
    pub fn from_bacnet_value(data: &[u8]) -> Result<PropertyValue, String> {
        use bacnet_rs::encoding::advanced::bitstring::decode_bit_string;
        use bacnet_rs::encoding::{
            decode_boolean, decode_character_string, decode_enumerated, decode_real, decode_signed,
            decode_unsigned, decode_object_identifier,
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

    /// Convert bacnet-rs PropertyValue to baccy-core PropertyValue
    pub fn convert_bacnet_property_value(
        value: &bacnet_rs::property::PropertyValue,
    ) -> Result<PropertyValue, String> {
        use bacnet_rs::property::PropertyValue as BacnetPropertyValue;

        match value {
            BacnetPropertyValue::Null => {
                Err("Null property values are not supported".to_string())
            }
            BacnetPropertyValue::Boolean(b) => Ok(PropertyValue::Boolean(*b)),
            BacnetPropertyValue::Unsigned(u) => Ok(PropertyValue::Unsigned(*u)),
            BacnetPropertyValue::Signed(i) => Ok(PropertyValue::Integer(*i)),
            BacnetPropertyValue::Real(f) => Ok(PropertyValue::Real(*f)),
            BacnetPropertyValue::Double(d) => Ok(PropertyValue::Real(*d as f32)),
            BacnetPropertyValue::OctetString(bytes) => {
                // Convert octet string to hex string representation
                let hex_string = bytes.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                Ok(PropertyValue::String(hex_string))
            }
            BacnetPropertyValue::CharacterString(s) => Ok(PropertyValue::String(s.clone())),
            BacnetPropertyValue::BitString(bits) => Ok(PropertyValue::BitString(bits.clone())),
            BacnetPropertyValue::Enumerated(e) => Ok(PropertyValue::Enumerated(*e)),
            BacnetPropertyValue::Date(_, _, _, _) => {
                Err("Date property values are not yet supported".to_string())
            }
            BacnetPropertyValue::Time(_, _, _, _) => {
                Err("Time property values are not yet supported".to_string())
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

/// Format an ObjectId for error messages
fn format_object_id(object: &ObjectId) -> String {
    format!("{:?}({})", object.object_type, object.instance)
}

/// Format a PropertyId for error messages
fn format_property_id(property: &PropertyId) -> String {
    format!("{:?}", property)
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
            service_choice: UnconfirmedServiceChoice::WhoIs,
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
            service_choice: UnconfirmedServiceChoice::WhoIs,
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
        use type_conversion::{convert_bacnet_property_value, to_bacnet_object_type, to_bacnet_property_id};

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
            service_choice: ConfirmedServiceChoice::ReadProperty,
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
                    service_choice,
                    service_data,
                    ..
                } => {
                    // Verify this is a ReadProperty response
                    if service_choice != ConfirmedServiceChoice::ReadProperty {
                        continue;
                    }
                    
                    // Use ReadPropertyResponse::decode() to parse the response
                    use bacnet_rs::service::ReadPropertyResponse;
                    
                    let response = ReadPropertyResponse::decode(&service_data).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to parse property read response for {} property {} from device {}: {}. \
                            The device may have returned malformed data or an unsupported encoding.",
                            format_object_id(&object), format_property_id(&property), device, e
                        ))
                    })?;
                    
                    // Extract the first property value from the response
                    // In bacnet-rs 0.3, property_values is Vec<PropertyValue>
                    if response.property_values.is_empty() {
                        return Err(ProtocolError::DecodingError(format!(
                            "Property read returned no values for {} property {} from device {}. \
                            The property may not be implemented or the device returned an empty response.",
                            format_object_id(&object), format_property_id(&property), device
                        )));
                    }
                    
                    // Convert bacnet-rs PropertyValue to baccy-core PropertyValue
                    let bacnet_value = &response.property_values[0];
                    let property_value = convert_bacnet_property_value(bacnet_value).map_err(|e| {
                        ProtocolError::DecodingError(format!(
                            "Failed to convert property value for {} property {} from device {}: {}. \
                            The property type may not be supported or the value is invalid.",
                            format_object_id(&object), format_property_id(&property), device, e
                        ))
                    })?;

                    return Ok(property_value);
                }
                Apdu::Error {
                    service_choice,
                    error_class,
                    error_code,
                    ..
                } => {
                    // Verify this is a ReadProperty error
                    if service_choice != ConfirmedServiceChoice::ReadProperty {
                        continue;
                    }
                    
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
                Apdu::Reject {
                    reject_reason,
                    ..
                } => {
                    return Err(ProtocolError::DecodingError(format!(
                        "Request rejected by device {}: {:?}",
                        device, reject_reason
                    )));
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
            WritePropertyRequest::new(bacnet_object_id, bacnet_property_id.into(), property_value_bytes);

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
            service_choice: ConfirmedServiceChoice::WriteProperty,
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

            // Decode NPDU and APDU for proper response handling
            let mut offset = 0;
            if response_bytes.len() >= 4 && response_bytes[0] == 0x81 {
                offset = 4;
            }

            if response_bytes.len() <= offset {
                continue;
            }

            let (_, npdu_len) = Npdu::decode(&response_bytes[offset..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode NPDU: {}", e))
            })?;
            offset += npdu_len;

            let apdu = Apdu::decode(&response_bytes[offset..]).map_err(|e| {
                ProtocolError::DecodingError(format!("Failed to decode APDU: {}", e))
            })?;

            match apdu {
                Apdu::SimpleAck { service_choice, .. } => {
                    // Verify this is a WriteProperty acknowledgment
                    if service_choice == ConfirmedServiceChoice::WriteProperty as u8 {
                        return Ok(());
                    }
                }
                Apdu::Error {
                    service_choice,
                    error_class,
                    error_code,
                    ..
                } => {
                    // Verify this is a WriteProperty error
                    if service_choice != ConfirmedServiceChoice::WriteProperty {
                        continue;
                    }
                    
                    return Err(ProtocolError::BacnetError {
                        class: match error_class {
                            0 => ErrorClass::Device,
                            1 => ErrorClass::Object,
                            2 => ErrorClass::Property,
                            _ => ErrorClass::Unknown(error_class as u8),
                        },
                        code: match error_code {
                            40 => ErrorCode::WriteAccessDenied,
                            _ => ErrorCode::Unknown(error_code as u8),
                        },
                    });
                }
                Apdu::Reject {
                    reject_reason,
                    ..
                } => {
                    return Err(ProtocolError::DecodingError(format!(
                        "WriteProperty request rejected by device {}: {:?}",
                        device, reject_reason
                    )));
                }
                _ => continue,
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
        let property_ref = PropertyReference::new(PropertyIdentifier::ObjectList);

        // Create read access specification
        let read_spec = ReadAccessSpecification::new(device_object, vec![property_ref]);

        // Create ReadPropertyMultiple request
        let rpm_request = ReadPropertyMultipleRequest::new(vec![read_spec]);

        // Encode the request
        let mut service_data = Vec::new();
        for spec in &rpm_request.read_access_specifications {
            // Object identifier - context tag 0
            // Use ObjectType enum and convert to u32 for encoding
            let obj_type_num: u32 = spec.object_identifier.object_type.into();
            let object_id: u32 = (obj_type_num << 22) | spec.object_identifier.instance;
            service_data.push(0x0C);
            service_data.extend_from_slice(&object_id.to_be_bytes());

            // Property references - context tag 1
            service_data.push(0x1E);
            for prop_ref in &spec.property_references {
                service_data.push(0x09);
                // Convert PropertyIdentifier to u32 for encoding
                let prop_id_value: u32 = prop_ref.property_identifier.into();
                service_data.push(prop_id_value as u8);

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
            service_choice: ConfirmedServiceChoice::ReadPropertyMultiple,
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
                    service_choice,
                    service_data,
                    ..
                } => {
                    if invoke_id != 1 {
                        continue;
                    }
                    
                    // Verify this is a ReadPropertyMultiple response
                    if service_choice != ConfirmedServiceChoice::ReadPropertyMultiple {
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

                            // Decode using ObjectIdentifier's From<u32> implementation
                            let bacnet_obj_id: BacnetObjectId = obj_id_raw.into();
                            
                            // Convert bacnet-rs ObjectType to baccy-core ObjectType
                            if let Some(obj_type) =
                                type_conversion::from_bacnet_object_type(bacnet_obj_id.object_type)
                            {
                                let object_id = ObjectId {
                                    object_type: obj_type,
                                    instance: bacnet_obj_id.instance,
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
                Apdu::Error {
                    service_choice,
                    error_class,
                    error_code,
                    ..
                } => {
                    // Verify this is a ReadPropertyMultiple error
                    if service_choice != ConfirmedServiceChoice::ReadPropertyMultiple {
                        continue;
                    }
                    
                    return Err(ProtocolError::BacnetError {
                        class: match error_class {
                            0 => ErrorClass::Device,
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
                Apdu::Reject {
                    reject_reason,
                    ..
                } => {
                    return Err(ProtocolError::DecodingError(format!(
                        "ReadPropertyMultiple request rejected by device {}: {:?}",
                        device, reject_reason
                    )));
                }
                _ => {
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_rs::property::PropertyValue as BacnetPropertyValue;
    use bacnet_rs::object::{ObjectIdentifier, ObjectType as BacnetObjectType};
    use bacnet_rs::object::property_identifier::PropertyIdentifier;
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
            PropertyValue::ObjectIdentifier { object_type, instance } => {
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
        assert!(result.unwrap_err().contains("Null property values are not supported"));
    }

    #[test]
    fn test_convert_bacnet_property_value_unknown() {
        let bacnet_value = BacnetPropertyValue::Unknown(vec![0x01, 0x02]);
        let result = type_conversion::convert_bacnet_property_value(&bacnet_value);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown property value type is not supported"));
    }

    #[test]
    fn test_read_property_response_parsing() {
        // Create a mock ReadPropertyResponse with multiple values
        let obj_id = ObjectIdentifier::new(BacnetObjectType::AnalogInput, 1);
        let property_values = vec![
            BacnetPropertyValue::Real(72.5),
            BacnetPropertyValue::Unsigned(100),
        ];
        
        let response = ReadPropertyResponse::new(
            obj_id,
            PropertyIdentifier::PresentValue,
            property_values,
        );

        // Verify we can extract the first value
        assert!(!response.property_values.is_empty());
        assert_eq!(response.property_values.len(), 2);
        
        // Convert the first value
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

    // Tests for Task 7.2: MS/TP address caching

    #[test]
    fn test_cache_mstp_address() {
        use std::sync::Arc;
        use std::time::Duration;

        // Create a mock transport that returns MS/TP addresses
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

        // Create a mock I-Am response with MS/TP source address
        let device_id = 12345u32;
        let vendor_id = 42u16;
        
        // Encode a minimal I-Am message
        // BVLC header (4 bytes)
        let mut iam_data = vec![0x81, 0x0A, 0x00, 0x00];
        
        // NPDU (minimal)
        iam_data.extend_from_slice(&[0x01, 0x00]);
        
        // APDU: Unconfirmed-Request I-Am
        iam_data.push(0x10); // Unconfirmed-Request
        iam_data.push(0x00); // I-Am service choice
        
        // I-Am payload: device object identifier
        iam_data.push(0xC4); // Object identifier tag
        let obj_id = (8u32 << 22) | device_id; // Device object type (8) + instance
        iam_data.extend_from_slice(&obj_id.to_be_bytes());
        
        // Max APDU length
        iam_data.push(0x21); // Unsigned tag
        iam_data.push(0x05); // 1024 bytes
        
        // Segmentation support
        iam_data.push(0x91); // Enumerated tag
        iam_data.push(0x03); // No segmentation
        
        // Vendor ID
        iam_data.push(0x22); // Unsigned tag (2 bytes)
        iam_data.extend_from_slice(&vendor_id.to_be_bytes());
        
        // Update BVLC length
        let total_len = iam_data.len() as u16;
        iam_data[2] = (total_len >> 8) as u8;
        iam_data[3] = (total_len & 0xFF) as u8;
        
        // Create MS/TP source address
        let mstp_address = Address::MsTp { network: 0, mac: 42 };
        
        // Create mock transport with the I-Am response
        let mock_transport = Arc::new(MockMstpTransport {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![
                (mstp_address.clone(), iam_data),
            ]))),
        });
        
        // Create BacnetService
        let service = BacnetService::new(mock_transport, Duration::from_secs(1));
        
        // Receive I-Am (this should cache the MS/TP address)
        let device = service.receive_iam(Duration::from_millis(100)).unwrap();
        assert_eq!(device.instance, device_id);
        
        // Verify the address was cached by trying to get it
        let cached_address = service.get_device_address(device_id).unwrap();
        assert_eq!(cached_address, mstp_address);
        
        // Verify it's specifically an MS/TP address
        match cached_address {
            Address::MsTp { network, mac } => {
                assert_eq!(network, 0);
                assert_eq!(mac, 42);
            }
            _ => panic!("Expected MS/TP address"),
        }
    }

    #[test]
    fn test_cache_ip_address() {
        use std::sync::Arc;
        use std::time::Duration;

        // Create a mock transport that returns IP addresses
        struct MockIpTransport {
            responses: Arc<Mutex<VecDeque<(Address, Vec<u8>)>>>,
        }

        impl crate::Transport for MockIpTransport {
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
                Address::Ip("0.0.0.0:47808".parse().unwrap())
            }
        }

        // Create a mock I-Am response with IP source address
        let device_id = 54321u32;
        let vendor_id = 99u16;
        
        // Encode a minimal I-Am message (same as above)
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
        
        // Create IP source address
        let ip_address = Address::Ip("192.168.1.100:47808".parse().unwrap());
        
        // Create mock transport with the I-Am response
        let mock_transport = Arc::new(MockIpTransport {
            responses: Arc::new(Mutex::new(VecDeque::from(vec![
                (ip_address.clone(), iam_data),
            ]))),
        });
        
        // Create BacnetService
        let service = BacnetService::new(mock_transport, Duration::from_secs(1));
        
        // Receive I-Am (this should cache the IP address)
        let device = service.receive_iam(Duration::from_millis(100)).unwrap();
        assert_eq!(device.instance, device_id);
        
        // Verify the address was cached
        let cached_address = service.get_device_address(device_id).unwrap();
        assert_eq!(cached_address, ip_address);
        
        // Verify it's specifically an IP address
        match cached_address {
            Address::Ip(socket_addr) => {
                assert_eq!(socket_addr.to_string(), "192.168.1.100:47808");
            }
            _ => panic!("Expected IP address"),
        }
    }

    #[test]
    fn test_get_device_address_not_found() {
        use std::sync::Arc;
        use std::time::Duration;

        struct MockTransport;

        impl crate::Transport for MockTransport {
            fn send(&self, _address: &Address, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }

            fn broadcast(&self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }

            fn receive(&self, _timeout: Duration) -> Result<(Address, Vec<u8>), TransportError> {
                Err(TransportError::Timeout)
            }

            fn local_address(&self) -> Address {
                Address::Ip("0.0.0.0:47808".parse().unwrap())
            }
        }

        let service = BacnetService::new(Arc::new(MockTransport), Duration::from_secs(1));
        
        // Try to get address for a device that hasn't been discovered
        let result = service.get_device_address(99999);
        assert!(result.is_err());
        
        match result {
            Err(ProtocolError::DecodingError(msg)) => {
                assert!(msg.contains("Unknown device"));
                assert!(msg.contains("99999"));
                assert!(msg.contains("Who-Is/I-Am"));
            }
            _ => panic!("Expected DecodingError for unknown device"),
        }
    }

    #[test]
    fn test_cache_multiple_devices() {
        use std::sync::Arc;
        use std::time::Duration;

        struct MockTransport {
            responses: Arc<Mutex<VecDeque<(Address, Vec<u8>)>>>,
        }

        impl crate::Transport for MockTransport {
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
                Address::Ip("0.0.0.0:47808".parse().unwrap())
            }
        }

        // Helper function to create I-Am message
        fn create_iam_message(device_id: u32, vendor_id: u16) -> Vec<u8> {
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
            iam_data
        }

        // Create responses for multiple devices with different address types
        let responses = vec![
            (
                Address::MsTp { network: 0, mac: 10 },
                create_iam_message(1000, 1),
            ),
            (
                Address::Ip("192.168.1.101:47808".parse().unwrap()),
                create_iam_message(2000, 2),
            ),
            (
                Address::MsTp { network: 0, mac: 20 },
                create_iam_message(3000, 3),
            ),
        ];

        let mock_transport = Arc::new(MockTransport {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
        });

        let service = BacnetService::new(mock_transport, Duration::from_secs(1));

        // Receive and cache all three devices
        let device1 = service.receive_iam(Duration::from_millis(100)).unwrap();
        assert_eq!(device1.instance, 1000);

        let device2 = service.receive_iam(Duration::from_millis(100)).unwrap();
        assert_eq!(device2.instance, 2000);

        let device3 = service.receive_iam(Duration::from_millis(100)).unwrap();
        assert_eq!(device3.instance, 3000);

        // Verify all addresses are cached correctly
        let addr1 = service.get_device_address(1000).unwrap();
        match addr1 {
            Address::MsTp { network, mac } => {
                assert_eq!(network, 0);
                assert_eq!(mac, 10);
            }
            _ => panic!("Expected MS/TP address for device 1000"),
        }

        let addr2 = service.get_device_address(2000).unwrap();
        match addr2 {
            Address::Ip(socket_addr) => {
                assert_eq!(socket_addr.to_string(), "192.168.1.101:47808");
            }
            _ => panic!("Expected IP address for device 2000"),
        }

        let addr3 = service.get_device_address(3000).unwrap();
        match addr3 {
            Address::MsTp { network, mac } => {
                assert_eq!(network, 0);
                assert_eq!(mac, 20);
            }
            _ => panic!("Expected MS/TP address for device 3000"),
        }
    }
}
