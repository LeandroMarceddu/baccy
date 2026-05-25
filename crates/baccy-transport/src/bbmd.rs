use crate::{Address, Transport, TransportError};
use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A registered foreign device entry in the FDT (Foreign Device Table)
#[derive(Debug, Clone)]
struct ForeignDeviceEntry {
    address: SocketAddr,
    time_to_live: u32,
    registration_time: Instant,
}

/// A peer BBMD entry in the BDT (Broadcast Distribution Table)
#[derive(Debug, Clone)]
struct BdtEntry {
    address: SocketAddr,
    ip: [u8; 4],
    port: u16,
    subnet_mask: [u8; 4],
}

/// Configuration for BBMD operation
#[derive(Debug, Clone)]
pub struct BbmdConfig {
    /// Enable BBMD server functionality (accept foreign registrations, forward broadcasts)
    pub enabled: bool,
    /// If set, register as a foreign device with this remote BBMD
    pub register_with_bbmd: Option<SocketAddr>,
    /// Registration TTL in seconds (default 120)
    pub registration_ttl: u32,
}

impl Default for BbmdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            register_with_bbmd: None,
            registration_ttl: 120,
        }
    }
}

/// A BACnet/IP transport with BBMD (BACnet Broadcast Management Device) support.
///
/// This transport owns a UDP socket and handles all BVLC message types:
///   - 0x0A / 0x0B: Original-Unicast-NPDU / Original-Broadcast-NPDU (passthrough)
///   - 0x04: Forwarded-NPDU (extracts inner NPDU+APDU)
///   - 0x05: Register-Foreign-Device (updates FDT)
///   - 0x01 / 0x02 / 0x03: Write/Read-BDT and Read-BDT-Ack
///   - 0x06 / 0x07 / 0x08: Read-FDT, Read-FDT-Ack, Delete-FDT
///
/// When BBMD is disabled (`config.enabled = false`) it behaves identically to
/// `BacnetIpTransport` — all BBMD-specific messages are silently ignored.
pub struct BbmdTransport {
    socket: UdpSocket,
    local_addr: SocketAddr,
    broadcast_addr: SocketAddr,
    config: BbmdConfig,
    fdt: Mutex<HashMap<SocketAddr, ForeignDeviceEntry>>,
    bdt: Mutex<Vec<BdtEntry>>,
    last_registration: Mutex<Option<Instant>>,
    bbmd_address: Mutex<Option<SocketAddr>>,
}

impl BbmdTransport {
    // BVLC function codes (BACnet standard, ASHRAE 135 Annex J)

    /// BVLC-Result
    pub const BVLC_TYPE_RESULT: u8 = 0x00;
    /// Write-Broadcast-Distribution-Table
    pub const BVLC_TYPE_WRITE_BDT: u8 = 0x01;
    /// Read-Broadcast-Distribution-Table
    pub const BVLC_TYPE_READ_BDT: u8 = 0x02;
    /// Read-Broadcast-Distribution-Table-Ack
    pub const BVLC_TYPE_READ_BDT_ACK: u8 = 0x03;
    /// Forwarded-NPDU
    pub const BVLC_TYPE_FORWARDED: u8 = 0x04;
    /// Register-Foreign-Device
    pub const BVLC_TYPE_REGISTER: u8 = 0x05;
    /// Read-Foreign-Device-Table
    pub const BVLC_TYPE_READ_FDT: u8 = 0x06;
    /// Read-Foreign-Device-Table-Ack
    pub const BVLC_TYPE_READ_FDT_ACK: u8 = 0x07;
    /// Delete-Foreign-Device-Table-Entry
    pub const BVLC_TYPE_DELETE_FDT: u8 = 0x08;
    /// Distribute-Broadcast-To-Network
    pub const BVLC_TYPE_DISTRIBUTE: u8 = 0x09;
    /// Original-Unicast-NPDU
    pub const BVLC_TYPE_UNICAST: u8 = 0x0A;
    /// Original-Broadcast-NPDU
    pub const BVLC_TYPE_BROADCAST: u8 = 0x0B;

    /// Default BACnet/IP port (0xBAC0)
    pub const DEFAULT_PORT: u16 = 47808;

    /// Create a new BBMD transport bound to the given address.
    pub fn bind(bind_addr: SocketAddr, config: BbmdConfig) -> Result<Self, TransportError> {
        let socket = UdpSocket::bind(bind_addr)
            .map_err(|e| TransportError::BindFailed(e))?;
        socket.set_broadcast(true)
            .map_err(|e| TransportError::BindFailed(e))?;

        let local_addr = socket.local_addr()
            .map_err(|e| TransportError::BindFailed(e))?;

        let broadcast_addr = SocketAddr::from(([255, 255, 255, 255], bind_addr.port()));

        let register_with = config.register_with_bbmd;

        tracing::info!(
            local_addr = %local_addr,
            bbmd_enabled = config.enabled,
            register = ?register_with,
            "BBMD transport bound"
        );

        Ok(Self {
            socket,
            local_addr,
            broadcast_addr,
            config,
            fdt: Mutex::new(HashMap::new()),
            bdt: Mutex::new(Vec::new()),
            last_registration: Mutex::new(None),
            bbmd_address: Mutex::new(register_with),
        })
    }

    // -----------------------------------------------------------------
    // BVLC helpers
    // -----------------------------------------------------------------

    fn encode_bvlc_header(bvlc_type: u8, total_len: u16) -> [u8; 4] {
        [0x81, bvlc_type, (total_len >> 8) as u8, total_len as u8]
    }

    fn build_bvlc_packet(bvlc_type: u8, payload: &[u8]) -> Vec<u8> {
        let total_len = 4 + payload.len() as u16;
        let mut pkt = Vec::with_capacity(total_len as usize);
        pkt.extend_from_slice(&Self::encode_bvlc_header(bvlc_type, total_len));
        pkt.extend_from_slice(payload);
        pkt
    }

    fn send_bvlc_to(&self, dest: &SocketAddr, bvlc_type: u8, payload: &[u8]) -> Result<(), TransportError> {
        let pkt = Self::build_bvlc_packet(bvlc_type, payload);
        self.socket.send_to(&pkt, dest)
            .map_err(|e| TransportError::SendFailed(e))?;
        Ok(())
    }

    /// Send a BVLC-Result to the given address
    fn send_bvlc_result(&self, dest: &SocketAddr, code: u16) -> Result<(), TransportError> {
        self.send_bvlc_to(dest, Self::BVLC_TYPE_RESULT, &code.to_be_bytes())
    }

    // -----------------------------------------------------------------
    // FDT management
    // -----------------------------------------------------------------

    /// Remove expired foreign device entries
    fn expire_fdt(&self) {
        let mut fdt = self.fdt.lock().unwrap();
        let now = Instant::now();
        let before = fdt.len();
        fdt.retain(|addr, entry| {
            let elapsed = now.duration_since(entry.registration_time).as_secs() as u32;
            let keep = elapsed < entry.time_to_live;
            if !keep {
                tracing::debug!(address = %addr, "FDT entry expired");
            }
            keep
        });
        let after = fdt.len();
        if before != after {
            tracing::info!(removed = before - after, remaining = after, "FDT entries expired");
        }
    }

    // -----------------------------------------------------------------
    // Foreign device registration client
    // -----------------------------------------------------------------

    /// Send a Register-Foreign-Device message to the configured remote BBMD
    fn send_foreign_registration(&self) -> Result<(), TransportError> {
        let bbmd = self.bbmd_address.lock().unwrap();
        if let Some(bbmd_addr) = *bbmd {
            let ttl = self.config.registration_ttl;
            self.send_bvlc_to(&bbmd_addr, Self::BVLC_TYPE_REGISTER, &ttl.to_be_bytes())?;
            *self.last_registration.lock().unwrap() = Some(Instant::now());
            tracing::info!(bbmd = %bbmd_addr, ttl = ttl, "Sent foreign device registration");
        }
        Ok(())
    }

    /// Check if the foreign device registration needs to be renewed
    fn check_registration_renewal(&self) -> Result<(), TransportError> {
        if self.bbmd_address.lock().unwrap().is_none() {
            return Ok(());
        }

        let should_renew = match *self.last_registration.lock().unwrap() {
            Some(last) => {
                let elapsed = last.elapsed().as_secs() as u32;
                let renew_interval = (self.config.registration_ttl as f32 * 0.75) as u32;
                elapsed >= renew_interval
            }
            None => true,
        };

        if should_renew {
            self.send_foreign_registration()?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // BVLC message handler
    // -----------------------------------------------------------------

    /// Handle an incoming BVLC message.
    ///
    /// Returns `Some((address, data))` if the message should be passed up to
    /// the protocol layer, or `None` if it was handled internally (BBMD management).
    fn handle_bvlc(&self, data: &[u8], source: SocketAddr) -> Option<(Address, Vec<u8>)> {
        if data.len() < 4 || data[0] != 0x81 {
            return Some((Address::Ip(source), data.to_vec()));
        }

        let bvlc_type = data[1];
        let payload = if data.len() > 4 { &data[4..] } else { &[] };

        match bvlc_type {
            Self::BVLC_TYPE_UNICAST | Self::BVLC_TYPE_BROADCAST => {
                Some((Address::Ip(source), payload.to_vec()))
            }

            Self::BVLC_TYPE_FORWARDED | Self::BVLC_TYPE_DISTRIBUTE => {
                if payload.len() >= 6 {
                    let npdu = payload[6..].to_vec();
                    Some((Address::Ip(source), npdu))
                } else {
                    tracing::warn!(from = %source, "Malformed forwarded-NPDU");
                    None
                }
            }

            Self::BVLC_TYPE_REGISTER => {
                if !self.config.enabled {
                    return None;
                }
                if payload.len() >= 2 {
                    let ttl = u32::from(u16::from_be_bytes([payload[0], payload[1]]));
                    let mut fdt = self.fdt.lock().unwrap();
                    fdt.insert(source, ForeignDeviceEntry {
                        address: source,
                        time_to_live: ttl,
                        registration_time: Instant::now(),
                    });
                    let count = fdt.len();
                    tracing::info!(
                        from = %source,
                        ttl = ttl,
                        fdt_count = count,
                        "Foreign device registered"
                    );
                }
                None
            }

            Self::BVLC_TYPE_WRITE_BDT => {
                if !self.config.enabled {
                    return None;
                }
                let mut bdt = self.bdt.lock().unwrap();
                bdt.clear();
                let mut offset = 0;
                while offset + 10 <= payload.len() {
                    let ip = [
                        payload[offset],
                        payload[offset + 1],
                        payload[offset + 2],
                        payload[offset + 3],
                    ];
                    let port = u16::from_be_bytes([payload[offset + 4], payload[offset + 5]]);
                    let subnet = [
                        payload[offset + 6],
                        payload[offset + 7],
                        payload[offset + 8],
                        payload[offset + 9],
                    ];
                    let addr = SocketAddr::from((Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]), port));
                    bdt.push(BdtEntry { address: addr, ip, port, subnet_mask: subnet });
                    offset += 10;
                }
                tracing::info!(from = %source, count = bdt.len(), "BDT updated via Write-BDT");
                let _ = self.send_bvlc_result(&source, 0x0000);
                None
            }

            Self::BVLC_TYPE_READ_BDT => {
                if !self.config.enabled {
                    return None;
                }
                let bdt = self.bdt.lock().unwrap();
                let mut ack = Vec::with_capacity(bdt.len() * 10);
                for entry in bdt.iter() {
                    ack.extend_from_slice(&entry.ip);
                    ack.extend_from_slice(&entry.port.to_be_bytes());
                    ack.extend_from_slice(&entry.subnet_mask);
                }
                let _ = self.send_bvlc_to(&source, Self::BVLC_TYPE_READ_BDT_ACK, &ack);
                tracing::debug!(from = %source, count = bdt.len(), "Responded to Read-BDT");
                None
            }

            Self::BVLC_TYPE_READ_FDT => {
                if !self.config.enabled {
                    return None;
                }
                self.expire_fdt();
                let fdt = self.fdt.lock().unwrap();
                let mut ack = Vec::with_capacity(fdt.len() * 8);
                for (addr, entry) in fdt.iter() {
                    let ip_bytes = match addr.ip() {
                        std::net::IpAddr::V4(v4) => v4.octets(),
                        _ => continue,
                    };
                    ack.extend_from_slice(&ip_bytes);
                    ack.extend_from_slice(&addr.port().to_be_bytes());
                    let elapsed = entry.registration_time.elapsed().as_secs() as u32;
                    let remaining = entry.time_to_live.saturating_sub(elapsed) as u16;
                    ack.extend_from_slice(&remaining.to_be_bytes());
                }
                let _ = self.send_bvlc_to(&source, Self::BVLC_TYPE_READ_FDT_ACK, &ack);
                tracing::debug!(from = %source, count = fdt.len(), "Responded to Read-FDT");
                None
            }

            Self::BVLC_TYPE_DELETE_FDT => {
                if !self.config.enabled {
                    return None;
                }
                if payload.len() >= 6 {
                    let ip = Ipv4Addr::new(payload[0], payload[1], payload[2], payload[3]);
                    let port = u16::from_be_bytes([payload[4], payload[5]]);
                    let addr = SocketAddr::from((ip, port));
                    let mut fdt = self.fdt.lock().unwrap();
                    fdt.remove(&addr);
                    tracing::info!(from = %source, removed = %addr, "Foreign device deleted from FDT");
                }
                None
            }

            t @ (Self::BVLC_TYPE_RESULT | Self::BVLC_TYPE_READ_BDT_ACK | Self::BVLC_TYPE_READ_FDT_ACK) => {
                tracing::debug!(from = %source, type = t, "Ignoring unsolicited BVLC response");
                None
            }

            t => {
                tracing::warn!(from = %source, type = t, len = data.len(), "Unknown BVLC type");
                Some((Address::Ip(source), data.to_vec()))
            }
        }
    }

}

impl Transport for BbmdTransport {
    fn send(&self, address: &Address, data: &[u8]) -> Result<(), TransportError> {
        match address {
            Address::Ip(addr) => {
                let packet = Self::build_bvlc_packet(Self::BVLC_TYPE_UNICAST, data);
                self.socket.send_to(&packet, addr)
                    .map_err(|e| TransportError::SendFailed(e))?;
                tracing::debug!(dest = %addr, len = data.len(), "Sent unicast");
                Ok(())
            }
            Address::MsTp { .. } => {
                Err(TransportError::SendFailed(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Cannot send MS/TP address over BACnet/IP transport",
                )))
            }
        }
    }

    fn broadcast(&self, data: &[u8]) -> Result<(), TransportError> {
        // 1. Send Original-Broadcast-NPDU to the local network
        let pkt = Self::build_bvlc_packet(Self::BVLC_TYPE_BROADCAST, data);
        self.socket.send_to(&pkt, self.broadcast_addr)
            .map_err(|e| TransportError::SendFailed(e))?;

        tracing::debug!(len = data.len(), "Sent original broadcast");

        // 2. Forward to registered foreign devices (if BBMD enabled)
        if self.config.enabled {
            self.expire_fdt();

            let local_ip = match self.local_addr.ip() {
                std::net::IpAddr::V4(v4) => v4.octets(),
                _ => [0, 0, 0, 0],
            };

            // Build the Forwarded-NPDU payload once
            let mut forward_payload = Vec::with_capacity(6 + data.len());
            forward_payload.extend_from_slice(&local_ip);
            forward_payload.extend_from_slice(&self.local_addr.port().to_be_bytes());
            forward_payload.extend_from_slice(data);

            // Forward to FDT entries
            let fdt_snapshot = {
                let fdt = self.fdt.lock().unwrap();
                fdt.values().map(|e| e.address).collect::<Vec<_>>()
            };

            for fde_addr in &fdt_snapshot {
                let fwd_pkt = Self::build_bvlc_packet(Self::BVLC_TYPE_FORWARDED, &forward_payload);
                if let Err(e) = self.socket.send_to(&fwd_pkt, fde_addr) {
                    tracing::warn!(dest = %fde_addr, error = %e, "Failed to forward broadcast to FDE");
                } else {
                    tracing::debug!(dest = %fde_addr, "Forwarded broadcast to FDE");
                }
            }

            // Forward to BDT entries (peer BBMDs)
            let bdt_snapshot = {
                let bdt = self.bdt.lock().unwrap();
                bdt.iter().map(|e| e.address).collect::<Vec<_>>()
            };

            for peer_addr in &bdt_snapshot {
                let fwd_pkt = Self::build_bvlc_packet(Self::BVLC_TYPE_FORWARDED, &forward_payload);
                if let Err(e) = self.socket.send_to(&fwd_pkt, peer_addr) {
                    tracing::warn!(dest = %peer_addr, error = %e, "Failed to forward broadcast to peer BBMD");
                } else {
                    tracing::debug!(dest = %peer_addr, "Forwarded broadcast to peer BBMD");
                }
            }
        }

        Ok(())
    }

    fn receive(&self, timeout: Duration) -> Result<(Address, Vec<u8>), TransportError> {
        let start = Instant::now();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Err(TransportError::Timeout);
            }
            let remaining = timeout - elapsed;

            // Check and renew foreign registration if needed
            if self.config.enabled {
                let _ = self.check_registration_renewal();
            }

            self.socket.set_read_timeout(Some(remaining))
                .map_err(|e| TransportError::ReceiveFailed(e))?;

            let mut buffer = vec![0u8; 65535];
            match self.socket.recv_from(&mut buffer) {
                Ok((size, source)) => {
                    buffer.truncate(size);
                    if let Some(result) = self.handle_bvlc(&buffer, source) {
                        tracing::debug!(from = %source, len = result.1.len(), "Received message");
                        return Ok(result);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                    continue;
                }
                Err(e) => {
                    return Err(TransportError::ReceiveFailed(e));
                }
            }
        }
    }

    fn local_address(&self) -> Address {
        Address::Ip(self.local_addr)
    }
}
