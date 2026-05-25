use crate::{Address, Transport, TransportError};
use bacnet_rs::network::{NetworkAddress, Npdu};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A route table entry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// Destination network number
    pub network: u16,
    /// Next-hop address on the connected network (None = use DADR from NPDU)
    pub next_hop: Option<Address>,
    /// Transport interface index (into RouterTransport's interfaces vec)
    pub interface: usize,
}

/// Router transport that manages multiple BACnet network interfaces and
/// routes messages between them according to DNET/DADR/SLR rules.
///
/// Each interface is associated with a local BACnet network number.
/// The route table maps remote network numbers to (next_hop, interface).
///
/// Network-layer messages (Who-Is-Router-To-Network, I-Am-Router-To-Network, etc.)
/// are handled internally. Regular messages with DNET are forwarded to the correct
/// interface. Messages without DNET are delivered locally.
pub struct RouterTransport {
    interfaces: Vec<(String, Arc<dyn Transport>, u16)>,
    routes: Mutex<Vec<RouteEntry>>,
}

impl RouterTransport {
    /// Create a new router with no interfaces or routes
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            routes: Mutex::new(Vec::new()),
        }
    }

    /// Add a transport interface with a name and local network number.
    /// Returns the interface index for use with `add_route`.
    pub fn add_interface(
        &mut self,
        name: &str,
        network: u16,
        transport: Arc<dyn Transport>,
    ) -> usize {
        let idx = self.interfaces.len();
        self.interfaces
            .push((name.to_string(), transport, network));
        idx
    }

    /// Add a route to a remote network via a specific interface.
    pub fn add_route(&self, network: u16, next_hop: Option<Address>, interface: usize) {
        self.routes.lock().unwrap().push(RouteEntry {
            network,
            next_hop,
            interface,
        });
    }

    /// Remove a route to a network
    pub fn remove_route(&self, network: u16) {
        self.routes.lock().unwrap().retain(|r| r.network != network);
    }

    /// Get a copy of all routes
    pub fn routes(&self) -> Vec<RouteEntry> {
        self.routes.lock().unwrap().clone()
    }

    /// Get local network numbers for all interfaces
    pub fn local_networks(&self) -> Vec<u16> {
        self.interfaces.iter().map(|(_, _, n)| *n).collect()
    }

    /// Get a reference to all interfaces
    pub fn interfaces(&self) -> &[(String, Arc<dyn Transport>, u16)] {
        &self.interfaces
    }
}

// ---------- internal helpers ----------

impl RouterTransport {
    fn find_interface_by_network(&self, network: u16) -> Option<usize> {
        self.interfaces.iter().position(|(_, _, n)| *n == network)
    }

    fn find_route(&self, dnet: u16) -> Option<RouteEntry> {
        self.routes
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.network == dnet)
            .cloned()
    }

    fn decode_npdu(data: &[u8]) -> Result<(Npdu, usize), ()> {
        Npdu::decode(data).map_err(|_| ())
    }

    fn get_dnet(data: &[u8]) -> Option<u16> {
        Self::decode_npdu(data)
            .ok()
            .and_then(|(npdu, _)| npdu.destination)
            .map(|d| d.network)
    }

    fn get_snet(data: &[u8]) -> Option<u16> {
        Self::decode_npdu(data)
            .ok()
            .and_then(|(npdu, _)| npdu.source)
            .map(|s| s.network)
    }

    fn is_network_message(data: &[u8]) -> bool {
        data.len() >= 2 && (data[1] & 0x80) != 0
    }
}

// ---------- network-layer message handling ----------

impl RouterTransport {
    /// Handle an incoming network-layer message (control byte bit 7 set).
    /// Returns `true` if the message was consumed internally.
    fn handle_network_message(
        &self,
        data: &[u8],
        source: &Address,
        iface_idx: usize,
    ) -> bool {
        let Ok((_npdu, npdu_len)) = Npdu::decode(data) else {
            return false;
        };
        let payload = &data[npdu_len..];
        if payload.is_empty() {
            return false;
        }

        let msg_type = payload[0];
        let msg_data = if payload.len() > 1 {
            &payload[1..]
        } else {
            &[]
        };

        match msg_type {
            0x00 => {
                // Who-Is-Router-To-Network
                if msg_data.len() >= 2 {
                    let dnet = u16::from_be_bytes([msg_data[0], msg_data[1]]);
                    if self.local_networks().contains(&dnet) || self.find_route(dnet).is_some() {
                        self.send_iam_router(source, iface_idx, dnet);
                    }
                } else if msg_data.is_empty() {
                    // Asking for all routes
                    let mut nets: Vec<u16> = self.local_networks();
                    nets.extend(self.routes.lock().unwrap().iter().map(|r| r.network));
                    nets.sort();
                    nets.dedup();
                    for &net in &nets {
                        self.send_iam_router(source, iface_idx, net);
                    }
                }
                true
            }
            0x01 => {
                // I-Am-Router-To-Network — learn routes
                if msg_data.len() >= 2 && msg_data.len() % 2 == 0 {
                    for chunk in msg_data.chunks(2) {
                        if chunk.len() == 2 {
                            let dnet = u16::from_be_bytes([chunk[0], chunk[1]]);
                            if !self.local_networks().contains(&dnet) {
                                tracing::info!(
                                    "Learned route to network {} via {:?} on interface {}",
                                    dnet,
                                    source,
                                    iface_idx
                                );
                                self.add_route(dnet, Some(source.clone()), iface_idx);
                            }
                        }
                    }
                }
                true
            }
            0x04 => {
                tracing::info!("Router-Busy-To-Network for {:?}", msg_data);
                true
            }
            0x05 => {
                tracing::info!("Router-Available-To-Network for {:?}", msg_data);
                true
            }
            _ => {
                tracing::warn!("Unhandled network message type: 0x{:02x}", msg_type);
                true
            }
        }
    }

    /// Send an I-Am-Router-To-Network message (type 0x01) on the specified interface
    fn send_iam_router(&self, dest: &Address, iface_idx: usize, dnet: u16) {
        let local_net = self.interfaces[iface_idx].2;
        let local_addr = self.interfaces[iface_idx].1.local_address();

        let mut npdu = Npdu::new();
        npdu.control.network_message = true;
        npdu.control.expecting_reply = false;

        let src_bytes = addr_to_bytes(&local_addr);
        npdu.source = Some(NetworkAddress::new(local_net, src_bytes));
        npdu.control.source_present = true;

        let mut data = npdu.encode();
        data.push(0x01);
        data.extend_from_slice(&dnet.to_be_bytes());

        if let Err(e) = self.interfaces[iface_idx].1.send(dest, &data) {
            tracing::warn!("Failed to send I-Am-Router-To-Network: {}", e);
        }
    }

    /// Add SNET/SADR to a message received from a remote network so the
    /// application layer can identify the originating network.
    fn add_source_routing(&self, data: &[u8], local_net: u16, source_addr: &Address) -> Vec<u8> {
        let Ok((npdu, _)) = Npdu::decode(data) else {
            return data.to_vec();
        };
        if npdu.source.is_some() {
            return data.to_vec();
        }

        let mut npdu = npdu;
        npdu.source = Some(NetworkAddress::new(local_net, addr_to_bytes(source_addr)));
        npdu.control.source_present = true;
        npdu.encode()
    }

    /// Forward a message to a remote network, decrementing hop count.
    /// Returns true if forwarded successfully.
    fn forward_to_network(&self, data: &[u8], _source: &Address, dnet: u16) -> bool {
        let route = match self.find_route(dnet) {
            Some(r) => r,
            None => {
                tracing::debug!("No route to network {}", dnet);
                return false;
            }
        };
        let Some((_name, transport, _)) = self.interfaces.get(route.interface) else {
            return false;
        };

        let Ok((mut npdu, _)) = Npdu::decode(data) else {
            return false;
        };

        let hc = npdu.hop_count.unwrap_or(255);
        if hc == 0 {
            tracing::warn!("Hop count exceeded for network {}", dnet);
            return false;
        }
        npdu.hop_count = Some(hc - 1);

        let next_hop = route.next_hop.as_ref().cloned().unwrap_or_else(|| {
            npdu.destination
                .as_ref()
                .map(|d| netaddr_to_addr(&d.address))
                .unwrap_or_else(|| local_addr_of(&self.interfaces, route.interface))
        });

        let encoded = npdu.encode();
        match transport.send(&next_hop, &encoded) {
            Ok(()) => {
                tracing::debug!(
                    "Forwarded to network {} (hop={}) via interface {}",
                    dnet,
                    hc - 1,
                    route.interface
                );
                true
            }
            Err(e) => {
                tracing::warn!("Forward to network {} failed: {}", dnet, e);
                false
            }
        }
    }
}

// ---------- Transport trait ----------

impl Transport for RouterTransport {
    fn send(&self, address: &Address, data: &[u8]) -> Result<(), TransportError> {
        let existing_dnet = Self::get_dnet(data);

        if let Some(dnet) = existing_dnet {
            // NPDU already has a destination — route it
            if let Some(idx) = self.find_interface_by_network(dnet) {
                return self.interfaces[idx].1.send(address, data);
            }
            if let Some(route) = self.find_route(dnet) {
                if let Some((_, t, _)) = self.interfaces.get(route.interface) {
                    let next_hop = route.next_hop.as_ref().unwrap_or(address);
                    return t.send(next_hop, data);
                }
            }
            return Err(TransportError::SendFailed(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("No route to network {}", dnet),
            )));
        }

        // No DNET — determine destination network from the address
        let dest_network = match address {
            Address::MsTp { network, .. } => *network,
            _ => 0,
        };

        if dest_network == 0 || self.local_networks().contains(&dest_network) {
            // Local delivery
            if let Some(idx) = self.find_interface_by_network(dest_network) {
                return self.interfaces[idx].1.send(address, data);
            }
            if let Some(idx) = self.interfaces.iter().position(|(_, t, _)| {
                std::mem::discriminant(&t.local_address())
                    == std::mem::discriminant(address)
            }) {
                return self.interfaces[idx].1.send(address, data);
            }
            if let Some((_, t, _)) = self.interfaces.first() {
                return t.send(address, data);
            }
        } else {
            // Remote network — add DNET and forward
            if let Some(route) = self.find_route(dest_network) {
                if let Some((_, t, _)) = self.interfaces.get(route.interface) {
                    let Ok((mut npdu, _)) = Npdu::decode(data) else {
                        return Err(TransportError::SendFailed(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Failed to decode NPDU for routing",
                        )));
                    };
                    let dadr = addr_to_bytes(address);
                    npdu.destination = Some(NetworkAddress::new(dest_network, dadr));
                    npdu.control.destination_present = true;
                    if npdu.hop_count.is_none() {
                        npdu.hop_count = Some(255);
                    }
                    let next_hop = route.next_hop.as_ref().unwrap_or(address);
                    let encoded = npdu.encode();
                    return t.send(next_hop, &encoded);
                }
            }
            return Err(TransportError::SendFailed(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("No route to network {}", dest_network),
            )));
        }

        Err(TransportError::SendFailed(io::Error::new(
            io::ErrorKind::NotConnected,
            "No transport interfaces",
        )))
    }

    fn broadcast(&self, data: &[u8]) -> Result<(), TransportError> {
        let dnet = Self::get_dnet(data);

        // Network-specific broadcast
        if let Some(net) = dnet {
            if net == 0xFFFF || net == 0 {
                // Global / local broadcast — send on all interfaces
                return self.broadcast_all(data);
            }
            if let Some(idx) = self.find_interface_by_network(net) {
                return self.interfaces[idx].1.broadcast(data);
            }
            if let Some(route) = self.find_route(net) {
                if let Some((_, t, _)) = self.interfaces.get(route.interface) {
                    return t.broadcast(data);
                }
            }
            return Err(TransportError::SendFailed(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("No route to network {} for broadcast", net),
            )));
        }

        // No DNET — broadcast on all interfaces
        self.broadcast_all(data)
    }

    fn receive(&self, timeout: Duration) -> Result<(Address, Vec<u8>), TransportError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(50);

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Err(TransportError::Timeout);
            }

            for (iface_idx, (name, transport, local_net)) in
                self.interfaces.iter().enumerate()
            {
                match transport.receive(poll_interval) {
                    Ok((addr, data)) => {
                        // Network-layer messages are handled internally
                        if Self::is_network_message(&data) {
                            self.handle_network_message(&data, &addr, iface_idx);
                            continue;
                        }

                        let dnet = Self::get_dnet(&data);

                        // Forward messages destined for other networks
                        if let Some(net) = dnet {
                            if !self.local_networks().contains(&net) {
                                self.forward_to_network(&data, &addr, net);
                                continue;
                            }
                            // DNET matches our network — deliver to application
                            return Ok((addr, data));
                        }

                        // No DNET — deliver to application.
                        // If this message came from a remote network (e.g. an MS/TP
                        // interface with non-zero network), add SNET/SADR so the
                        // application knows the origin.
                        if Self::get_snet(&data).is_none() && *local_net != 0 {
                            let modified =
                                self.add_source_routing(&data, *local_net, &addr);
                            return Ok((addr, modified));
                        }

                        return Ok((addr, data));
                    }
                    Err(TransportError::Timeout) => continue,
                    Err(e) => {
                        tracing::trace!("Receive error on {}: {}", name, e);
                        continue;
                    }
                }
            }
        }
    }

    fn local_address(&self) -> Address {
        self.interfaces
            .first()
            .map(|(_, t, _)| t.local_address())
            .unwrap_or(Address::MsTp { network: 0, mac: 0 })
    }
}

impl RouterTransport {
    fn broadcast_all(&self, data: &[u8]) -> Result<(), TransportError> {
        let mut last_err = None;
        let mut success = false;
        for (name, transport, _) in &self.interfaces {
            match transport.broadcast(data) {
                Ok(()) => success = true,
                Err(e) => {
                    tracing::warn!("Broadcast failed on {}: {}", name, e);
                    last_err = Some(e);
                }
            }
        }
        if success {
            Ok(())
        } else if let Some(e) = last_err {
            Err(e)
        } else {
            Err(TransportError::SendFailed(io::Error::new(
                io::ErrorKind::NotConnected,
                "No transport interfaces",
            )))
        }
    }
}

impl std::fmt::Debug for RouterTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ifaces: Vec<(&str, u16)> = self
            .interfaces
            .iter()
            .map(|(n, _, net)| (n.as_str(), *net))
            .collect();
        let route_count = self.routes.lock().unwrap().len();
        f.debug_struct("RouterTransport")
            .field("interfaces", &ifaces)
            .field("route_count", &route_count)
            .finish()
    }
}

// ---------- helpers ----------

fn addr_to_bytes(addr: &Address) -> Vec<u8> {
    match addr {
        Address::Ip(sa) => {
            let mut bytes = match sa.ip() {
                std::net::IpAddr::V4(v4) => v4.octets().to_vec(),
                std::net::IpAddr::V6(v6) => v6.octets().to_vec(),
            };
            bytes.extend_from_slice(&sa.port().to_be_bytes());
            bytes
        }
        Address::MsTp { mac, .. } => vec![*mac],
    }
}

fn netaddr_to_addr(data: &[u8]) -> Address {
    if data.len() == 6 {
        let ip = std::net::Ipv4Addr::new(data[0], data[1], data[2], data[3]);
        let port = u16::from_be_bytes([data[4], data[5]]);
        Address::Ip(std::net::SocketAddr::new(std::net::IpAddr::V4(ip), port))
    } else if data.len() == 1 {
        Address::MsTp {
            network: 0,
            mac: data[0],
        }
    } else {
        Address::MsTp {
            network: 0,
            mac: 0,
        }
    }
}

fn local_addr_of(
    interfaces: &[(String, Arc<dyn Transport>, u16)],
    idx: usize,
) -> Address {
    interfaces
        .get(idx)
        .map(|(_, t, _)| t.local_address())
        .unwrap_or(Address::MsTp { network: 0, mac: 0 })
}
