// Application logic layer

mod device_manager;
mod object_manager;
mod property_manager;
mod trending_manager;

pub use device_manager::DeviceManager;
pub use object_manager::ObjectManager;
pub use property_manager::{parse_property_value, PropertyManager};
pub use trending_manager::{DataPoint, TrendedProperty, TrendingManager};

use baccy_core::{DeviceId, ObjectId, PropertyId};
use baccy_protocol::ProtocolError;

/// Errors that can occur during application operations
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Protocol layer error
    #[error("Protocol error: {0}")]
    ProtocolError(#[from] ProtocolError),

    /// Device not found in discovered devices list
    #[error("Device not found: {0}")]
    DeviceNotFound(DeviceId),

    /// Object not found in device's object list
    #[error("Object not found: {0:?}")]
    ObjectNotFound(ObjectId),

    /// Property not found in object's property list
    #[error("Property not found: {0:?}")]
    PropertyNotFound(PropertyId),
}

impl AppError {
    /// Format a user-friendly error message for GUI display
    ///
    /// Returns a concise, human-readable description of the error
    /// suitable for displaying to end users, with context about
    /// the operation that failed.
    pub fn user_message(&self) -> String {
        match self {
            AppError::ProtocolError(e) => e.user_message(),
            AppError::DeviceNotFound(device_id) => {
                format!(
                    "Device {} not found. Try refreshing the device list.",
                    device_id
                )
            }
            AppError::ObjectNotFound(object_id) => {
                format!(
                    "Object {} (instance {}) not found. The object may have been removed from the device.",
                    object_id.object_type.name(),
                    object_id.instance
                )
            }
            AppError::PropertyNotFound(property_id) => {
                format!(
                    "Property {} not found. The property may not be supported by this object.",
                    property_id.name()
                )
            }
        }
    }

    /// Format an error message with operation context
    ///
    /// Returns a detailed error message that includes information about
    /// what operation was being performed when the error occurred.
    pub fn with_context(&self, operation: &str) -> String {
        format!("{}: {}", operation, self.user_message())
    }

    /// Format an error message with full context (operation, device, object, property)
    ///
    /// Returns a comprehensive error message suitable for logging that includes
    /// all available context information.
    pub fn with_full_context(
        &self,
        operation: &str,
        device_id: Option<DeviceId>,
        object_id: Option<ObjectId>,
        property_id: Option<PropertyId>,
    ) -> String {
        let mut parts = vec![operation.to_string()];

        if let Some(dev_id) = device_id {
            parts.push(format!("device {}", dev_id));
        }

        if let Some(obj_id) = object_id {
            parts.push(format!(
                "object {} (instance {})",
                obj_id.object_type.name(),
                obj_id.instance
            ));
        }

        if let Some(prop_id) = property_id {
            parts.push(format!("property {}", prop_id.name()));
        }

        format!("{}: {}", parts.join(" > "), self.user_message())
    }
}
