use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub avg_response_time_ms: f64,
}

const MAX_RESPONSE_TIMES: usize = 100;

struct StatsData {
    packets_sent: u64,
    packets_received: u64,
    bytes_sent: u64,
    bytes_received: u64,
    errors: u64,
    response_times: Vec<Duration>,
}

pub struct StatsCollector {
    inner: Mutex<StatsData>,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(StatsData {
                packets_sent: 0,
                packets_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                errors: 0,
                response_times: Vec::with_capacity(MAX_RESPONSE_TIMES),
            }),
        }
    }

    pub fn record_send(&self, bytes: usize) {
        let mut data = self.inner.lock().unwrap();
        data.packets_sent += 1;
        data.bytes_sent += bytes as u64;
    }

    pub fn record_receive(&self, bytes: usize) {
        let mut data = self.inner.lock().unwrap();
        data.packets_received += 1;
        data.bytes_received += bytes as u64;
    }

    pub fn record_error(&self) {
        let mut data = self.inner.lock().unwrap();
        data.errors += 1;
    }

    pub fn record_response_time(&self, duration: Duration) {
        let mut data = self.inner.lock().unwrap();
        data.response_times.push(duration);
        if data.response_times.len() > MAX_RESPONSE_TIMES {
            data.response_times.remove(0);
        }
    }

    pub fn snapshot(&self) -> NetworkStats {
        let data = self.inner.lock().unwrap();
        let avg_ms = if data.response_times.is_empty() {
            0.0
        } else {
            let total_ms: f64 = data
                .response_times
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum();
            total_ms / data.response_times.len() as f64
        };
        NetworkStats {
            packets_sent: data.packets_sent,
            packets_received: data.packets_received,
            bytes_sent: data.bytes_sent,
            bytes_received: data.bytes_received,
            errors: data.errors,
            avg_response_time_ms: (avg_ms * 100.0).round() / 100.0,
        }
    }
}
