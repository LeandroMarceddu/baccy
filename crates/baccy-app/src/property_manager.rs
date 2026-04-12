// Property manager for managing BACnet properties of an object

use crate::AppError;
use baccy_core::{DataType, DeviceId, ObjectId, Property, PropertyId, PropertyValue};
use baccy_protocol::BacnetService;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Tracks property value changes for highlighting purposes
///
/// When a property value changes, it is highlighted for 3 seconds with a fade animation.
/// The opacity starts at 1.0 and fades to 0.0 over the duration.
#[derive(Debug)]
pub struct HighlightTracker {
    /// Map of property ID to (previous value, highlight start time)
    highlights: HashMap<PropertyId, (PropertyValue, Instant)>,
    /// Duration of the highlight fade animation (3 seconds)
    fade_duration: Duration,
}

impl HighlightTracker {
    /// Create a new HighlightTracker with 3-second fade duration
    pub fn new() -> Self {
        Self {
            highlights: HashMap::new(),
            fade_duration: Duration::from_secs(3),
        }
    }

    /// Check if a property value has changed and start highlighting if so
    ///
    /// # Arguments
    /// * `property_id` - The property ID to check
    /// * `new_value` - The new property value
    ///
    /// # Returns
    /// true if the value changed and highlighting was started, false otherwise
    pub fn check_change(&mut self, property_id: PropertyId, new_value: &PropertyValue) -> bool {
        // Check if we have a previous value
        if let Some((prev_value, _)) = self.highlights.get(&property_id) {
            // Compare values
            if prev_value != new_value {
                // Value changed - update highlight start time
                self.highlights
                    .insert(property_id, (new_value.clone(), Instant::now()));
                return true;
            }
        } else {
            // First time seeing this property - store it but don't highlight
            self.highlights
                .insert(property_id, (new_value.clone(), Instant::now() - self.fade_duration));
        }
        false
    }

    /// Get the current highlight opacity for a property
    ///
    /// Returns a value between 0.0 (no highlight) and 1.0 (full highlight).
    /// The opacity fades linearly from 1.0 to 0.0 over the fade duration.
    ///
    /// # Arguments
    /// * `property_id` - The property ID to get opacity for
    ///
    /// # Returns
    /// The opacity value between 0.0 and 1.0
    pub fn get_opacity(&self, property_id: PropertyId) -> f32 {
        if let Some((_, start_time)) = self.highlights.get(&property_id) {
            let elapsed = start_time.elapsed();
            if elapsed < self.fade_duration {
                // Calculate opacity: 1.0 at start, 0.0 at end
                let progress = elapsed.as_secs_f32() / self.fade_duration.as_secs_f32();
                1.0 - progress
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl Default for HighlightTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages properties of a BACnet object
pub struct PropertyManager {
    properties: HashMap<PropertyId, Property>,
    service: Arc<BacnetService>,
    highlight_tracker: HighlightTracker,
}

impl PropertyManager {
    /// Create a new PropertyManager
    ///
    /// # Arguments
    /// * `service` - The BACnet service to use for property operations
    ///
    /// # Returns
    /// A new `PropertyManager` instance
    pub fn new(service: Arc<BacnetService>) -> Self {
        Self {
            properties: HashMap::new(),
            service,
            highlight_tracker: HighlightTracker::new(),
        }
    }

    /// Load properties from a device/object
    ///
    /// Retrieves all readable properties for the specified object and stores them
    /// in the internal HashMap. This replaces any previously loaded properties.
    ///
    /// # Arguments
    /// * `device` - The device ID to load properties from
    /// * `object` - The object ID to load properties from
    ///
    /// # Returns
    /// Ok(()) if properties were loaded successfully
    ///
    /// # Errors
    /// Returns `AppError::ProtocolError` if properties cannot be retrieved
    pub fn load_properties(&mut self, device: DeviceId, object: ObjectId) -> Result<(), AppError> {
        // Clear existing properties
        self.properties.clear();

        // Select properties based on object type
        let property_ids = match object.object_type {
            baccy_core::ObjectType::Device => {
                // Device objects have different properties than I/O objects
                vec![
                    PropertyId::ObjectName,
                    // Device-specific properties would go here
                    // PropertyId::VendorName, PropertyId::ModelName, etc.
                ]
            }
            baccy_core::ObjectType::AnalogInput
            | baccy_core::ObjectType::AnalogOutput
            | baccy_core::ObjectType::AnalogValue => {
                // Analog objects
                vec![
                    PropertyId::ObjectName,
                    PropertyId::PresentValue,
                    PropertyId::Description,
                    PropertyId::Units,
                    PropertyId::StatusFlags,
                ]
            }
            baccy_core::ObjectType::BinaryInput
            | baccy_core::ObjectType::BinaryOutput
            | baccy_core::ObjectType::BinaryValue => {
                // Binary objects
                vec![
                    PropertyId::ObjectName,
                    PropertyId::PresentValue,
                    PropertyId::Description,
                    PropertyId::StatusFlags,
                ]
            }
            baccy_core::ObjectType::MultiStateInput
            | baccy_core::ObjectType::MultiStateOutput
            | baccy_core::ObjectType::MultiStateValue => {
                // Multi-state objects
                vec![
                    PropertyId::ObjectName,
                    PropertyId::PresentValue,
                    PropertyId::Description,
                    PropertyId::StatusFlags,
                ]
            }
        };

        for property_id in property_ids {
            // Try to read each property
            match self.service.read_property(device, object, property_id) {
                Ok(value) => {
                    // Determine data type from the value
                    let data_type = match &value {
                        PropertyValue::Real(_) => DataType::Real,
                        PropertyValue::Integer(_) => DataType::Integer,
                        PropertyValue::Unsigned(_) => DataType::Unsigned,
                        PropertyValue::Boolean(_) => DataType::Boolean,
                        PropertyValue::String(_) => DataType::CharacterString,
                        PropertyValue::Enumerated(_) => DataType::Enumerated,
                        PropertyValue::BitString(_) => DataType::BitString,
                    };

                    // Determine if property is writable (simplified logic)
                    let writable = matches!(
                        property_id,
                        PropertyId::PresentValue | PropertyId::Description
                    );

                    let property = Property {
                        id: property_id,
                        name: format!("{:?}", property_id),
                        value: value.clone(),
                        data_type,
                        writable,
                    };

                    // Check if value changed for highlighting
                    self.highlight_tracker.check_change(property_id, &value);

                    self.properties.insert(property_id, property);
                }
                Err(e) => {
                    // Skip properties that can't be read
                    // Description and StatusFlags are optional, so only log at debug level for them
                    // Also, if we get a BacnetError with UnknownProperty, it's expected for optional properties
                    let is_optional_property = matches!(
                        property_id,
                        PropertyId::Description | PropertyId::StatusFlags
                    );

                    let is_unknown_property = matches!(
                        &e,
                        baccy_protocol::ProtocolError::BacnetError { code, .. }
                        if matches!(code, baccy_protocol::ErrorCode::UnknownProperty)
                    );

                    if is_optional_property || is_unknown_property {
                        tracing::debug!(
                            device_id = device,
                            object_type = ?object.object_type,
                            object_instance = object.instance,
                            property_id = ?property_id,
                            "Property not available (optional or not supported by this object)"
                        );
                    } else {
                        tracing::warn!(
                            device_id = device,
                            object_type = ?object.object_type,
                            object_instance = object.instance,
                            property_id = ?property_id,
                            error = %e,
                            "Failed to read property from object"
                        );
                    }
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Get a property by its ID
    ///
    /// # Arguments
    /// * `id` - The property ID to look up
    ///
    /// # Returns
    /// A reference to the property if found, None otherwise
    pub fn get_property(&self, id: PropertyId) -> Option<&Property> {
        self.properties.get(&id)
    }

    /// Update a property value from a string input
    ///
    /// Parses the string input based on the property's data type and writes it to the device.
    ///
    /// # Arguments
    /// * `device` - The device ID to write to
    /// * `object` - The object ID to write to
    /// * `property` - The property ID to write
    /// * `value_str` - The string value to parse and write
    ///
    /// # Returns
    /// Ok(()) if the property was updated successfully
    ///
    /// # Errors
    /// Returns `AppError::PropertyNotFound` if the property doesn't exist
    /// Returns `AppError::ProtocolError` if parsing or writing fails
    pub fn update_property(
        &mut self,
        device: DeviceId,
        object: ObjectId,
        property: PropertyId,
        value_str: &str,
    ) -> Result<(), AppError> {
        // Get the property to determine its data type
        let prop = self.properties.get(&property).ok_or_else(|| {
            let error = AppError::PropertyNotFound(property);
            tracing::error!(
                device_id = device,
                object_type = ?object.object_type,
                object_instance = object.instance,
                property_id = ?property,
                error = %error,
                "Property not found in cache during update"
            );
            error
        })?;

        // Check if property is writable
        if !prop.writable {
            let error = AppError::ProtocolError(baccy_protocol::ProtocolError::EncodingError(
                format!("Property {:?} is read-only", property),
            ));
            tracing::error!(
                device_id = device,
                object_type = ?object.object_type,
                object_instance = object.instance,
                property_id = ?property,
                error = %error,
                "Attempted to write to read-only property"
            );
            return Err(error);
        }

        // Parse the value based on the property's data type
        let value = parse_property_value(value_str, prop.data_type)?;

        // Write the property value
        if let Err(e) = self
            .service
            .write_property(device, object, property, value.clone())
        {
            tracing::error!(
                device_id = device,
                object_type = ?object.object_type,
                object_instance = object.instance,
                property_id = ?property,
                value = ?value,
                error = %e,
                "Failed to write property value to device"
            );
            return Err(e.into());
        }

        // Update the cached value
        if let Some(prop) = self.properties.get_mut(&property) {
            prop.value = value;
        }

        tracing::info!(
            device_id = device,
            object_type = ?object.object_type,
            object_instance = object.instance,
            property_id = ?property,
            value = value_str,
            "Property value updated successfully"
        );

        Ok(())
    }

    /// Refresh all properties for an object
    ///
    /// Reloads all properties from the device, updating the cached values.
    ///
    /// # Arguments
    /// * `device` - The device ID to refresh from
    /// * `object` - The object ID to refresh from
    ///
    /// # Returns
    /// Ok(()) if properties were refreshed successfully
    ///
    /// # Errors
    /// Returns `AppError::ProtocolError` if the refresh fails
    pub fn refresh(&mut self, device: DeviceId, object: ObjectId) -> Result<(), AppError> {
        // Reload all properties
        self.load_properties(device, object)
    }

    /// Get the highlight opacity for a property
    ///
    /// Returns a value between 0.0 (no highlight) and 1.0 (full highlight).
    ///
    /// # Arguments
    /// * `property_id` - The property ID to get opacity for
    ///
    /// # Returns
    /// The opacity value between 0.0 and 1.0
    pub fn get_highlight_opacity(&self, property_id: PropertyId) -> f32 {
        self.highlight_tracker.get_opacity(property_id)
    }
}

/// Parse a string value into a PropertyValue based on the data type
///
/// # Arguments
/// * `input` - The string input to parse
/// * `data_type` - The expected data type
///
/// # Returns
/// Ok(PropertyValue) if parsing succeeds, Err(AppError) otherwise
pub fn parse_property_value(input: &str, data_type: DataType) -> Result<PropertyValue, AppError> {
    match data_type {
        DataType::Real => {
            let value = input.parse::<f32>().map_err(|_| {
                AppError::ProtocolError(baccy_protocol::ProtocolError::EncodingError(
                    format!("Invalid real number format: '{}'", input),
                ))
            })?;
            Ok(PropertyValue::Real(value))
        }
        DataType::Integer => {
            let value = input.parse::<i32>().map_err(|_| {
                AppError::ProtocolError(baccy_protocol::ProtocolError::EncodingError(
                    format!("Invalid integer format: '{}'", input),
                ))
            })?;
            Ok(PropertyValue::Integer(value))
        }
        DataType::Unsigned => {
            let value = input.parse::<u32>().map_err(|_| {
                AppError::ProtocolError(baccy_protocol::ProtocolError::EncodingError(
                    format!("Invalid unsigned integer format: '{}'", input),
                ))
            })?;
            Ok(PropertyValue::Unsigned(value))
        }
        DataType::Boolean => {
            let value = match input.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => {
                    return Err(AppError::ProtocolError(
                        baccy_protocol::ProtocolError::EncodingError(format!(
                            "Invalid boolean format: '{}' (use true/false, 1/0, yes/no, on/off)",
                            input
                        )),
                    ))
                }
            };
            Ok(PropertyValue::Boolean(value))
        }
        DataType::CharacterString => Ok(PropertyValue::String(input.to_string())),
        DataType::Enumerated => {
            let value = input.parse::<u32>().map_err(|_| {
                AppError::ProtocolError(baccy_protocol::ProtocolError::EncodingError(
                    format!("Invalid enumerated value format: '{}'", input),
                ))
            })?;
            Ok(PropertyValue::Enumerated(value))
        }
        DataType::BitString => Err(AppError::ProtocolError(
            baccy_protocol::ProtocolError::EncodingError(
                "BitString editing not yet supported".to_string(),
            ),
        )),
    }
}

