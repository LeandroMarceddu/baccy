// Trending manager for tracking BACnet property values over time

use crate::AppError;
use baccy_core::{DeviceId, ObjectId, PropertyId, PropertyValue};
use baccy_protocol::BacnetService;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A single data point in the trending history
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// Time when the value was read
    pub timestamp: Instant,
    /// Property value (numeric only)
    pub value: f32,
}

/// A property being trended
#[derive(Debug, Clone)]
pub struct TrendedProperty {
    /// Device containing the property
    pub device_id: DeviceId,
    /// Object containing the property
    pub object_id: ObjectId,
    /// Property identifier
    pub property_id: PropertyId,
    /// Display name
    pub name: String,
    /// Engineering units
    pub units: String,
    /// Line color for chart (RGB)
    pub color: (u8, u8, u8),
    /// Visibility toggle
    pub visible: bool,
    /// Historical data points (max 100)
    pub history: VecDeque<DataPoint>,
}

/// Manages trending data for numeric properties
pub struct TrendingManager {
    /// Properties being trended
    properties: Vec<TrendedProperty>,
    /// Polling interval
    interval: Duration,
    /// Maximum history points per property
    max_points: usize,
    /// BACnet service for reading values
    service: Arc<BacnetService>,
    /// Last poll time
    last_poll: Option<Instant>,
}

impl TrendingManager {
    /// Create a new TrendingManager
    ///
    /// # Arguments
    /// * `service` - The BACnet service to use for reading properties
    /// * `interval` - Polling interval (default 5 seconds)
    ///
    /// # Returns
    /// A new `TrendingManager` instance
    pub fn new(service: Arc<BacnetService>) -> Self {
        Self {
            properties: Vec::new(),
            interval: Duration::from_secs(5),
            max_points: 100,
            service,
            last_poll: None,
        }
    }

    /// Add a property to trending
    ///
    /// Validates that the property is numeric before adding.
    ///
    /// # Arguments
    /// * `device_id` - The device ID
    /// * `object_id` - The object ID
    /// * `property_id` - The property ID
    /// * `name` - Display name for the property
    /// * `units` - Engineering units
    ///
    /// # Returns
    /// Ok(()) if the property was added successfully
    ///
    /// # Errors
    /// Returns `AppError::ProtocolError` if the property is not numeric
    pub fn add_property(
        &mut self,
        device_id: DeviceId,
        object_id: ObjectId,
        property_id: PropertyId,
        name: String,
        units: String,
    ) -> Result<(), AppError> {
        // Read the property to validate it's numeric
        let value = self.service.read_property(device_id, object_id, property_id)?;
        
        // Check if the value is numeric
        let numeric_value = match value {
            PropertyValue::Real(v) => v,
            PropertyValue::Integer(v) => v as f32,
            PropertyValue::Unsigned(v) => v as f32,
            _ => {
                return Err(AppError::ProtocolError(
                    baccy_protocol::ProtocolError::EncodingError(
                        format!("Property {:?} is not numeric", property_id),
                    ),
                ))
            }
        };

        // Assign a color based on the number of properties
        let color = Self::get_color_for_index(self.properties.len());

        // Create the trended property
        let mut trended_property = TrendedProperty {
            device_id,
            object_id,
            property_id,
            name,
            units,
            color,
            visible: true,
            history: VecDeque::new(),
        };

        // Add the initial data point
        trended_property.history.push_back(DataPoint {
            timestamp: Instant::now(),
            value: numeric_value,
        });

        self.properties.push(trended_property);

        tracing::info!(
            device_id,
            object_id = ?object_id,
            property_id = ?property_id,
            "Property added to trending"
        );

        Ok(())
    }

    /// Remove a property from trending
    ///
    /// # Arguments
    /// * `index` - The index of the property to remove
    pub fn remove_property(&mut self, index: usize) {
        if index < self.properties.len() {
            let prop = self.properties.remove(index);
            tracing::info!(
                device_id = prop.device_id,
                object_id = ?prop.object_id,
                property_id = ?prop.property_id,
                "Property removed from trending"
            );
        }
    }

    /// Clear all trending data
    pub fn clear_all(&mut self) {
        self.properties.clear();
        tracing::info!("All trending data cleared");
    }

    /// Toggle visibility of a property's line
    ///
    /// # Arguments
    /// * `index` - The index of the property to toggle
    pub fn toggle_visibility(&mut self, index: usize) {
        if let Some(prop) = self.properties.get_mut(index) {
            prop.visible = !prop.visible;
            tracing::debug!(
                device_id = prop.device_id,
                object_id = ?prop.object_id,
                property_id = ?prop.property_id,
                visible = prop.visible,
                "Property visibility toggled"
            );
        }
    }

    /// Check if it's time to poll
    ///
    /// # Returns
    /// true if enough time has elapsed since the last poll
    pub fn should_poll(&self) -> bool {
        match self.last_poll {
            Some(last) => last.elapsed() >= self.interval,
            None => !self.properties.is_empty(),
        }
    }

    /// Poll all properties and update history
    ///
    /// Reads the current value of each trended property and appends it to the history.
    /// Removes oldest points when exceeding max_points.
    ///
    /// # Returns
    /// Ok(()) if polling succeeded
    ///
    /// # Errors
    /// Returns `AppError::ProtocolError` if reading fails
    pub fn poll(&mut self) -> Result<(), AppError> {
        if self.properties.is_empty() {
            return Ok(());
        }

        let now = Instant::now();

        for prop in &mut self.properties {
            // Read the property value
            match self.service.read_property(prop.device_id, prop.object_id, prop.property_id) {
                Ok(value) => {
                    // Convert to numeric value
                    let numeric_value = match value {
                        PropertyValue::Real(v) => v,
                        PropertyValue::Integer(v) => v as f32,
                        PropertyValue::Unsigned(v) => v as f32,
                        _ => {
                            tracing::warn!(
                                device_id = prop.device_id,
                                object_id = ?prop.object_id,
                                property_id = ?prop.property_id,
                                "Property is no longer numeric, skipping"
                            );
                            continue;
                        }
                    };

                    // Add data point
                    prop.history.push_back(DataPoint {
                        timestamp: now,
                        value: numeric_value,
                    });

                    // Remove oldest points if exceeding max
                    while prop.history.len() > self.max_points {
                        prop.history.pop_front();
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        device_id = prop.device_id,
                        object_id = ?prop.object_id,
                        property_id = ?prop.property_id,
                        error = %e,
                        "Failed to read property for trending"
                    );
                    // Continue with other properties
                }
            }
        }

        self.last_poll = Some(now);

        Ok(())
    }

    /// Get the list of trended properties
    pub fn properties(&self) -> &[TrendedProperty] {
        &self.properties
    }

    /// Get a color for a property based on its index
    fn get_color_for_index(index: usize) -> (u8, u8, u8) {
        // Predefined color palette
        const COLORS: [(u8, u8, u8); 10] = [
            (255, 99, 132),   // Red
            (54, 162, 235),   // Blue
            (255, 206, 86),   // Yellow
            (75, 192, 192),   // Teal
            (153, 102, 255),  // Purple
            (255, 159, 64),   // Orange
            (199, 199, 199),  // Gray
            (83, 102, 255),   // Indigo
            (255, 99, 255),   // Pink
            (99, 255, 132),   // Green
        ];

        COLORS[index % COLORS.len()]
    }
}
