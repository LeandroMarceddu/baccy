// Object manager for managing BACnet objects within a device

use crate::AppError;
use baccy_core::{BacnetObject, DeviceId, ObjectId, ObjectType, PropertyId, PropertyValue};
use baccy_protocol::BacnetService;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages BACnet objects within a device
pub struct ObjectManager {
    objects: HashMap<ObjectId, BacnetObject>,
    service: Arc<BacnetService>,
}

impl ObjectManager {
    /// Create a new ObjectManager
    ///
    /// # Arguments
    /// * `service` - The BACnet service to use for object operations
    ///
    /// # Returns
    /// A new `ObjectManager` instance
    pub fn new(service: Arc<BacnetService>) -> Self {
        Self {
            objects: HashMap::new(),
            service,
        }
    }

    /// Load objects from a device
    ///
    /// Retrieves the object list from the specified device and stores the objects
    /// in the internal HashMap. This replaces any previously loaded objects.
    ///
    /// # Arguments
    /// * `device` - The device ID to load objects from
    ///
    /// # Returns
    /// Ok(()) if objects were loaded successfully
    ///
    /// # Errors
    /// Returns `AppError::ProtocolError` if the object list cannot be retrieved
    pub fn load_objects(&mut self, device: DeviceId) -> Result<(), AppError> {
        tracing::info!(device_id = device, "load_objects called for device");

        // Retrieve the object list from the device
        let object_ids = match self.service.read_object_list(device) {
            Ok(ids) => {
                tracing::info!(
                    device_id = device,
                    object_count = ids.len(),
                    "Successfully retrieved object list"
                );
                ids
            }
            Err(e) => {
                tracing::error!(
                    device_id = device,
                    error = %e,
                    "Failed to read object list from device"
                );
                return Err(e.into());
            }
        };

        // Clear existing objects
        self.objects.clear();

        tracing::info!(
            device_id = device,
            object_count = object_ids.len(),
            "Reading ObjectName for {} objects",
            object_ids.len()
        );

        for object_id in object_ids {
            tracing::debug!(
                device_id = device,
                object_type = ?object_id.object_type,
                instance = object_id.instance,
                "Adding object to manager"
            );

            let name = match self.service.read_property(
                device,
                object_id,
                PropertyId::ObjectName,
            ) {
                Ok(PropertyValue::String(name)) => name,
                _ => format!("{:?} {}", object_id.object_type, object_id.instance),
            };

            let object = BacnetObject {
                object_type: object_id.object_type,
                instance: object_id.instance,
                name,
            };
            self.objects.insert(object_id, object);
        }

        tracing::info!(
            device_id = device,
            total_objects = self.objects.len(),
            "load_objects completed successfully"
        );

        Ok(())
    }

    /// Get an object by its ID
    ///
    /// # Arguments
    /// * `id` - The object ID to look up
    ///
    /// # Returns
    /// A reference to the object if found, None otherwise
    pub fn get_object(&self, id: ObjectId) -> Option<&BacnetObject> {
        self.objects.get(&id)
    }

    /// List all objects
    ///
    /// # Returns
    /// A vector of references to all objects
    pub fn list_objects(&self) -> Vec<&BacnetObject> {
        self.objects.values().collect()
    }

    /// Group objects by their type
    ///
    /// Organizes objects by their ObjectType for tree view display.
    ///
    /// # Returns
    /// A HashMap mapping ObjectType to vectors of object references
    pub fn group_by_type(&self) -> HashMap<ObjectType, Vec<&BacnetObject>> {
        let mut grouped: HashMap<ObjectType, Vec<&BacnetObject>> = HashMap::new();

        for object in self.objects.values() {
            grouped
                .entry(object.object_type)
                .or_default()
                .push(object);
        }

        grouped
    }
}
