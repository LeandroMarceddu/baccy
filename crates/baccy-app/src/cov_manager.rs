use baccy_core::{DeviceId, ObjectId, PropertyId};
use baccy_protocol::{BacnetService, CovNotification, ProtocolError};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A registered COV subscription
struct CovSubscription {
    device_id: DeviceId,
    object_id: ObjectId,
    property_id: Option<PropertyId>,
    subscriber_process_id: u32,
    callback: Box<dyn Fn(CovNotification) + Send>,
}

impl std::fmt::Debug for CovSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CovSubscription")
            .field("device_id", &self.device_id)
            .field("object_id", &self.object_id)
            .field("property_id", &self.property_id)
            .field("subscriber_process_id", &self.subscriber_process_id)
            .finish()
    }
}

/// Manages COV subscriptions and notification routing
pub struct CovManager {
    service: Arc<BacnetService>,
    subscriptions: Mutex<HashMap<u32, CovSubscription>>,
    next_process_id: AtomicU32,
}

impl CovManager {
    /// Create a new CovManager
    pub fn new(service: Arc<BacnetService>) -> Self {
        Self {
            service,
            subscriptions: Mutex::new(HashMap::new()),
            next_process_id: AtomicU32::new(1),
        }
    }

    /// Subscribe for COV on an object.
    /// Returns the subscriber process ID (for unsubscribing later).
    pub fn subscribe(
        &self,
        device_id: DeviceId,
        object_id: ObjectId,
        lifetime: Option<u32>,
        callback: Box<dyn Fn(CovNotification) + Send>,
    ) -> Result<u32, ProtocolError> {
        let process_id = self.next_process_id.fetch_add(1, Ordering::SeqCst);

        self.service
            .subscribe_cov(device_id, object_id, process_id, lifetime, false)?;

        let sub = CovSubscription {
            device_id,
            object_id,
            property_id: None,
            subscriber_process_id: process_id,
            callback,
        };
        self.subscriptions.lock().unwrap().insert(process_id, sub);

        Ok(process_id)
    }

    /// Subscribe for COV on a specific property.
    pub fn subscribe_property(
        &self,
        device_id: DeviceId,
        object_id: ObjectId,
        property_id: PropertyId,
        lifetime: Option<u32>,
        cov_increment: Option<f32>,
        callback: Box<dyn Fn(CovNotification) + Send>,
    ) -> Result<u32, ProtocolError> {
        let process_id = self.next_process_id.fetch_add(1, Ordering::SeqCst);

        self.service.subscribe_cov_property(
            device_id,
            object_id,
            property_id,
            process_id,
            lifetime,
            false,
            cov_increment,
        )?;

        let sub = CovSubscription {
            device_id,
            object_id,
            property_id: Some(property_id),
            subscriber_process_id: process_id,
            callback,
        };
        self.subscriptions.lock().unwrap().insert(process_id, sub);

        Ok(process_id)
    }

    /// Cancel a subscription by process ID
    pub fn unsubscribe(&self, process_id: u32) -> Result<(), ProtocolError> {
        let sub = {
            let subs = self.subscriptions.lock().unwrap();
            subs.get(&process_id).cloned()
        };
        if let Some(sub) = sub {
            self.service
                .unsubscribe_cov(sub.device_id, sub.object_id, sub.subscriber_process_id)?;
            self.subscriptions.lock().unwrap().remove(&process_id);
        }
        Ok(())
    }

    /// Cancel all subscriptions for a given device+object
    pub fn unsubscribe_all(&self, device_id: DeviceId, object_id: ObjectId) -> Result<(), ProtocolError> {
        let ids: Vec<u32> = {
            let subs = self.subscriptions.lock().unwrap();
            subs.iter()
                .filter(|(_, s)| s.device_id == device_id && s.object_id == object_id)
                .map(|(id, _)| *id)
                .collect()
        };
        for id in ids {
            self.unsubscribe(id)?;
        }
        Ok(())
    }

    /// Poll for incoming COV notifications and route them to registered callbacks.
    /// Returns the number of notifications processed.
    pub fn poll_notifications(&self, timeout: Duration) -> Result<usize, ProtocolError> {
        let mut count = 0;
        loop {
            match self.service.receive_cov_notification(timeout)? {
                Some(notification) => {
                    let subs = self.subscriptions.lock().unwrap();
                    // Route to matching subscriptions
                    for sub in subs.values() {
                        // Match by process_id, device_id, and optionally property_id
                        if sub.subscriber_process_id == notification.subscriber_process_id
                            && sub.device_id == notification.device_id
                            && sub.object_id == notification.object_id
                        {
                            if let Some(ref prop_id) = sub.property_id {
                                // For property-specific subscriptions, only forward matching changes
                                let matches = notification.changed_values.iter().any(|(p, _)| p == prop_id);
                                if !matches {
                                    continue;
                                }
                            }
                            (sub.callback)(notification.clone());
                        }
                    }
                    count += 1;
                }
                None => break,
            }
        }
        Ok(count)
    }

    /// Get the BACnet service reference
    pub fn service(&self) -> &Arc<BacnetService> {
        &self.service
    }
}

// Need Clone for CovSubscription for the unsubscribe path
// We box the callback so we can implement Clone manually
impl Clone for CovSubscription {
    fn clone(&self) -> Self {
        // Since we can't clone Box<dyn Fn>, we create an empty/noop callback.
        // This is only used for reading subscription info, not for cloning callbacks.
        Self {
            device_id: self.device_id,
            object_id: self.object_id,
            property_id: self.property_id,
            subscriber_process_id: self.subscriber_process_id,
            callback: Box::new(|_| {}),
        }
    }
}
