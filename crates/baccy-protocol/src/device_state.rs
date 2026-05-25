use baccy_core::DeviceId;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct DeviceHealth {
    pub is_online: bool,
    pub consecutive_failures: u32,
    pub max_consecutive_failures: u32,
    pub last_success: Option<u64>,
    pub last_failure: Option<u64>,
}

pub struct DeviceTracker {
    devices: Mutex<HashMap<DeviceId, DeviceHealth>>,
    max_consecutive_failures: u32,
}

impl DeviceTracker {
    pub fn new(max_consecutive_failures: u32) -> Self {
        Self {
            devices: Mutex::new(HashMap::new()),
            max_consecutive_failures,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    pub fn record_success(&self, device_id: DeviceId) {
        let mut devices = self.devices.lock().unwrap();
        let health = devices.entry(device_id).or_insert(DeviceHealth {
            is_online: true,
            consecutive_failures: 0,
            max_consecutive_failures: self.max_consecutive_failures,
            last_success: None,
            last_failure: None,
        });
        health.is_online = true;
        health.consecutive_failures = 0;
        health.last_success = Some(Self::now());
    }

    pub fn record_failure(&self, device_id: DeviceId) {
        let mut devices = self.devices.lock().unwrap();
        let health = devices.entry(device_id).or_insert(DeviceHealth {
            is_online: true,
            consecutive_failures: 0,
            max_consecutive_failures: self.max_consecutive_failures,
            last_success: None,
            last_failure: None,
        });
        health.consecutive_failures += 1;
        health.last_failure = Some(Self::now());
        if health.consecutive_failures >= self.max_consecutive_failures {
            health.is_online = false;
        }
    }

    pub fn get_health(&self, device_id: DeviceId) -> Option<DeviceHealth> {
        self.devices.lock().unwrap().get(&device_id).cloned()
    }

    pub fn get_all_health(&self) -> HashMap<DeviceId, DeviceHealth> {
        self.devices.lock().unwrap().clone()
    }

    pub fn mark_online(&self, device_id: DeviceId) {
        let mut devices = self.devices.lock().unwrap();
        if let Some(health) = devices.get_mut(&device_id) {
            health.is_online = true;
            health.consecutive_failures = 0;
        }
    }
}
