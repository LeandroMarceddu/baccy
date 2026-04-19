// Device manager for discovering and managing BACnet devices

use crate::AppError;
use baccy_core::{Device, DeviceId};
use baccy_protocol::BacnetService;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Manages discovered BACnet devices and their metadata
pub struct DeviceManager {
    devices: HashMap<DeviceId, Device>,
    service: Arc<BacnetService>,
}

impl DeviceManager {
    /// Create a new DeviceManager
    ///
    /// # Arguments
    /// * `service` - The BACnet service to use for device discovery and communication
    ///
    /// # Returns
    /// A new `DeviceManager` instance
    pub fn new(service: Arc<BacnetService>) -> Self {
        Self {
            devices: HashMap::new(),
            service,
        }
    }

    /// Discover devices on the network
    ///
    /// Sends a Who-Is broadcast and collects I_Am responses. Duplicate device
    /// instances are handled by updating existing entries with the most recent
    /// information.
    ///
    /// # Returns
    /// Ok(()) if discovery completed successfully
    ///
    /// # Errors
    /// Returns `AppError::ProtocolError` if the Who-Is broadcast fails
    pub fn discover_devices(&mut self) -> Result<(), AppError> {
        // Send Who-Is broadcast
        if let Err(e) = self.service.who_is() {
            tracing::error!(
                error = %e,
                "Failed to send Who-Is broadcast during device discovery"
            );
            return Err(e.into());
        }

        // Collect I_Am responses for a reasonable timeout period
        // We'll try to receive multiple responses within a 5-second window
        let discovery_timeout = Duration::from_secs(5);
        let start_time = std::time::Instant::now();
        let mut device_count = 0;

        while start_time.elapsed() < discovery_timeout {
            // Try to receive an I_Am response with a short timeout
            let receive_timeout = Duration::from_millis(100);
            match self.service.receive_iam(receive_timeout) {
                Ok(device) => {
                    // Update or insert the device (handles duplicates)
                    self.update_device(device);
                    device_count += 1;
                    tracing::debug!(
                        device_count = device_count,
                        elapsed_ms = start_time.elapsed().as_millis(),
                        "Received I_Am response during device discovery"
                    );
                }
                Err(baccy_protocol::ProtocolError::Timeout) => {
                    // Timeout is expected when no more devices respond - don't log
                    continue;
                }
                Err(baccy_protocol::ProtocolError::TransportError(
                    baccy_transport::TransportError::Timeout,
                )) => {
                    // Transport timeout is also expected - don't log
                    continue;
                }
                Err(e) => {
                    // Only log actual errors (e.g., decode failures, transport errors)
                    tracing::warn!(
                        error = %e,
                        elapsed_ms = start_time.elapsed().as_millis(),
                        "Error receiving I_Am response during device discovery"
                    );
                    continue;
                }
            }
        }

        tracing::info!(
            device_count = device_count,
            "Device discovery completed"
        );

        Ok(())
    }

    /// Get a device by its device ID
    ///
    /// # Arguments
    /// * `id` - The device ID to look up
    ///
    /// # Returns
    /// A reference to the device if found, None otherwise
    pub fn get_device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.get(&id)
    }

    /// Update or insert a device
    ///
    /// If a device with the same instance number already exists, it will be
    /// updated with the new information. This handles duplicate device instances
    /// by keeping the most recent information.
    ///
    /// # Arguments
    /// * `device` - The device to update or insert
    pub fn update_device(&mut self, device: Device) {
        self.devices.insert(device.instance, device);
    }

    /// List all discovered devices
    ///
    /// # Returns
    /// A vector of references to all discovered devices
    pub fn list_devices(&self) -> Vec<&Device> {
        self.devices.values().collect()
    }
}
