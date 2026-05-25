// Packet inspection commands

use crate::state::AppState;
use baccy_transport::packet_log::PacketDirection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct PacketRecordInfo {
    pub timestamp_ms: u64,
    pub direction: String,
    pub source: String,
    pub destination: String,
    pub hex: String,
    pub length: usize,
}

fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 3);
    for (i, byte) in data.iter().enumerate() {
        if i > 0 && i % 16 == 0 {
            s.push('\n');
        } else if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

#[tauri::command]
pub fn get_packet_log(state: State<'_, AppState>) -> Result<Vec<PacketRecordInfo>, String> {
    let packet_log = state.packet_log.lock().unwrap();
    let log = packet_log.as_ref().ok_or("Packet log not initialized")?;
    let packets = log.get_packets();
    Ok(packets
        .into_iter()
        .map(|p| PacketRecordInfo {
            timestamp_ms: p.timestamp_ms,
            direction: if p.direction == PacketDirection::Sent {
                "sent"
            } else {
                "received"
            }
            .to_string(),
            source: p.source,
            destination: p.destination,
            hex: hex_encode(&p.data),
            length: p.length,
        })
        .collect())
}

#[tauri::command]
pub fn clear_packet_log(state: State<'_, AppState>) -> Result<(), String> {
    let packet_log = state.packet_log.lock().unwrap();
    if let Some(log) = packet_log.as_ref() {
        log.clear();
    }
    Ok(())
}

#[tauri::command]
pub fn set_packet_logging(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let packet_log = state.packet_log.lock().unwrap();
    if let Some(log) = packet_log.as_ref() {
        log.set_enabled(enabled);
    }
    Ok(())
}
