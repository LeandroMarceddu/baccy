use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

/// Per-device request throttle using a simple semaphore pattern.
/// Limits the number of concurrent in-flight requests to a single BACnet device.
pub struct RequestThrottle {
    max_concurrent: usize,
    state: Mutex<HashMap<u32, DeviceState>>,
}

#[derive(Default)]
struct DeviceState {
    current_count: usize,
}

impl RequestThrottle {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Acquire a permit, blocking until one is available
    pub fn acquire(&self, device_id: u32) {
        loop {
            let mut state = self.state.lock().unwrap();
            let dev = state.entry(device_id).or_default();
            if dev.current_count < self.max_concurrent {
                dev.current_count += 1;
                return;
            }
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Release a permit
    pub fn release(&self, device_id: u32) {
        let mut state = self.state.lock().unwrap();
        if let Some(dev) = state.get_mut(&device_id) {
            dev.current_count = dev.current_count.saturating_sub(1);
        }
    }

    /// Get current concurrency for a device
    pub fn current_concurrency(&self, device_id: u32) -> usize {
        self.state
            .lock()
            .unwrap()
            .get(&device_id)
            .map(|s| s.current_count)
            .unwrap_or(0)
    }

    /// Get max concurrent permits
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Get concurrency for all devices
    pub fn all_concurrency(&self) -> HashMap<u32, usize> {
        self.state
            .lock()
            .unwrap()
            .iter()
            .map(|(&id, s)| (id, s.current_count))
            .collect()
    }
}
