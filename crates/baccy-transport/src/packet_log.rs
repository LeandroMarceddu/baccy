use crate::{Address, Transport, TransportError};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A single captured packet
#[derive(Debug, Clone)]
pub struct PacketRecord {
    pub timestamp_ms: u64,
    pub direction: PacketDirection,
    pub source: String,
    pub destination: String,
    pub data: Vec<u8>,
    pub length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketDirection {
    Sent,
    Received,
}

/// Packet log buffer with configurable max size
pub struct PacketLog {
    packets: Mutex<VecDeque<PacketRecord>>,
    max_packets: usize,
    enabled: Mutex<bool>,
}

impl PacketLog {
    pub fn new(max_packets: usize) -> Self {
        Self {
            packets: Mutex::new(VecDeque::new()),
            max_packets,
            enabled: Mutex::new(true),
        }
    }

    pub fn record(
        &self,
        direction: PacketDirection,
        source: String,
        destination: String,
        data: &[u8],
    ) {
        if !*self.enabled.lock().unwrap() {
            return;
        }
        let mut packets = self.packets.lock().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        packets.push_back(PacketRecord {
            timestamp_ms: timestamp,
            direction,
            source,
            destination,
            data: data.to_vec(),
            length: data.len(),
        });
        while packets.len() > self.max_packets {
            packets.pop_front();
        }
    }

    pub fn get_packets(&self) -> Vec<PacketRecord> {
        self.packets.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear(&self) {
        self.packets.lock().unwrap().clear();
    }

    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
    }
}

/// Transport wrapper that logs all packets
pub struct LoggedTransport {
    inner: Arc<dyn Transport>,
    log: Arc<PacketLog>,
}

impl LoggedTransport {
    pub fn new(inner: Arc<dyn Transport>, log: Arc<PacketLog>) -> Self {
        Self { inner, log }
    }

    pub fn log(&self) -> &Arc<PacketLog> {
        &self.log
    }
}

impl Transport for LoggedTransport {
    fn send(&self, address: &Address, data: &[u8]) -> Result<(), TransportError> {
        self.log.record(
            PacketDirection::Sent,
            self.inner.local_address().to_string(),
            address.to_string(),
            data,
        );
        self.inner.send(address, data)
    }

    fn broadcast(&self, data: &[u8]) -> Result<(), TransportError> {
        self.log.record(
            PacketDirection::Sent,
            self.inner.local_address().to_string(),
            "Broadcast".to_string(),
            data,
        );
        self.inner.broadcast(data)
    }

    fn receive(&self, timeout: Duration) -> Result<(Address, Vec<u8>), TransportError> {
        let result = self.inner.receive(timeout)?;
        self.log.record(
            PacketDirection::Received,
            result.0.to_string(),
            self.inner.local_address().to_string(),
            &result.1,
        );
        Ok(result)
    }

    fn local_address(&self) -> Address {
        self.inner.local_address()
    }
}
