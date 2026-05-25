// Transport abstraction and implementations

pub mod bbmd;
pub mod frame;
pub mod mstp;
pub mod network_stats;
pub mod packet_log;
pub mod router;

use baccy_core::Address;
use std::io;
use std::time::Duration;

/// Errors that can occur during transport operations
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Failed to bind to the network address
    #[error("Failed to bind to address: {0}")]
    BindFailed(#[source] io::Error),

    /// Failed to send data
    #[error("Failed to send data: {0}")]
    SendFailed(#[source] io::Error),

    /// Failed to receive data
    #[error("Failed to receive data: {0}")]
    ReceiveFailed(#[source] io::Error),

    /// Operation timed out
    #[error("Operation timed out")]
    Timeout,
}

impl TransportError {
    /// Format a user-friendly error message for GUI display
    ///
    /// Returns a concise, human-readable description of the error
    /// suitable for displaying to end users.
    pub fn user_message(&self) -> String {
        match self {
            TransportError::BindFailed(e) => {
                format!("Unable to bind to network port. {}", Self::io_error_hint(e))
            }
            TransportError::SendFailed(e) => {
                format!("Failed to send message. {}", Self::io_error_hint(e))
            }
            TransportError::ReceiveFailed(e) => {
                format!("Failed to receive message. {}", Self::io_error_hint(e))
            }
            TransportError::Timeout => "No response received within timeout period.".to_string(),
        }
    }

    /// Get a helpful hint based on the I/O error kind
    fn io_error_hint(e: &io::Error) -> String {
        match e.kind() {
            io::ErrorKind::PermissionDenied => {
                "Check that you have permission to access the network.".to_string()
            }
            io::ErrorKind::AddrInUse => {
                "The port is already in use by another application.".to_string()
            }
            io::ErrorKind::AddrNotAvailable => "The network address is not available.".to_string(),
            io::ErrorKind::NetworkDown => "The network interface is down.".to_string(),
            io::ErrorKind::NetworkUnreachable => "The network is unreachable.".to_string(),
            _ => "Check your network connection and firewall settings.".to_string(),
        }
    }
}

/// Transport abstraction for BACnet communication
///
/// This trait provides a common interface for different BACnet transports
/// (BACnet/IP, MS/TP, etc.). Implementations must be thread-safe (Send + Sync)
/// to support use in async contexts.
pub trait Transport: Send + Sync {
    /// Send a BACnet message to a specific address
    ///
    /// # Arguments
    /// * `address` - The destination address
    /// * `data` - The BACnet message data to send
    ///
    /// # Errors
    /// Returns `TransportError::SendFailed` if the send operation fails
    fn send(&self, address: &Address, data: &[u8]) -> Result<(), TransportError>;

    /// Send a broadcast BACnet message
    ///
    /// # Arguments
    /// * `data` - The BACnet message data to broadcast
    ///
    /// # Errors
    /// Returns `TransportError::SendFailed` if the broadcast operation fails
    fn broadcast(&self, data: &[u8]) -> Result<(), TransportError>;

    /// Receive a BACnet message (blocking with timeout)
    ///
    /// # Arguments
    /// * `timeout` - Maximum time to wait for a message
    ///
    /// # Returns
    /// A tuple of (source address, message data) if a message is received
    ///
    /// # Errors
    /// Returns `TransportError::Timeout` if no message is received within the timeout
    /// Returns `TransportError::ReceiveFailed` if the receive operation fails
    fn receive(&self, timeout: Duration) -> Result<(Address, Vec<u8>), TransportError>;

    /// Get the local address of this transport
    ///
    /// # Returns
    /// The local address that this transport is bound to
    fn local_address(&self) -> Address;
}

/// BACnet/IP transport implementation using UDP
///
/// This transport binds to a UDP socket on the BACnet/IP port range (47808-47823)
/// and provides send/receive/broadcast capabilities for BACnet/IP communication.
pub struct BacnetIpTransport {
    socket: std::net::UdpSocket,
    broadcast_address: std::net::SocketAddr,
    local_address: std::net::SocketAddr,
}

impl BacnetIpTransport {
    /// Default BACnet/IP port (0xBAC0)
    pub const DEFAULT_PORT: u16 = 47808;

    /// Minimum BACnet/IP port (0xBAC0)
    pub const MIN_PORT: u16 = 47808;

    /// Maximum BACnet/IP port (0xBACF)
    pub const MAX_PORT: u16 = 47823;

    /// Create a new BACnet/IP transport bound to a specific address
    ///
    /// This is useful when you have multiple network interfaces and want to
    /// bind to a specific one (e.g., LAN vs WiFi).
    ///
    /// # Arguments
    /// * `bind_addr` - The socket address to bind to (IP:port)
    ///
    /// # Returns
    /// A new `BacnetIpTransport` instance
    ///
    /// # Errors
    /// Returns `TransportError::BindFailed` if:
    /// - The port is outside the valid BACnet/IP range
    /// - The socket cannot be bound (port already in use, permission denied, etc.)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use baccy_transport::BacnetIpTransport;
    /// use std::net::SocketAddr;
    ///
    /// // Bind to specific LAN interface
    /// let addr: SocketAddr = "192.168.0.194:47808".parse().unwrap();
    /// let transport = BacnetIpTransport::bind(addr)?;
    /// # Ok::<(), baccy_transport::TransportError>(())
    /// ```
    pub fn bind(bind_addr: std::net::SocketAddr) -> Result<Self, TransportError> {
        let bind_port = bind_addr.port();

        // Validate port is in BACnet/IP range
        if !(Self::MIN_PORT..=Self::MAX_PORT).contains(&bind_port) {
            let error = TransportError::BindFailed(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Port {} is outside BACnet/IP range ({}-{})",
                    bind_port,
                    Self::MIN_PORT,
                    Self::MAX_PORT
                ),
            ));
            tracing::error!(
                bind_port = bind_port,
                min_port = Self::MIN_PORT,
                max_port = Self::MAX_PORT,
                error = %error,
                "Port is outside valid BACnet/IP range"
            );
            return Err(error);
        }

        let socket = match std::net::UdpSocket::bind(bind_addr) {
            Ok(s) => s,
            Err(e) => {
                let error = TransportError::BindFailed(e);
                tracing::error!(
                    bind_addr = %bind_addr,
                    error = %error,
                    "Failed to bind UDP socket"
                );
                return Err(error);
            }
        };

        // Enable broadcast on the socket
        if let Err(e) = socket.set_broadcast(true) {
            let error = TransportError::BindFailed(e);
            tracing::error!(
                bind_addr = %bind_addr,
                error = %error,
                "Failed to enable broadcast on socket"
            );
            return Err(error);
        }

        // Configure broadcast address
        // Always use global broadcast for BACnet/IP
        let broadcast_address = std::net::SocketAddr::from(([255, 255, 255, 255], bind_port));

        // Get the actual local address
        let local_address = match socket.local_addr() {
            Ok(addr) => addr,
            Err(e) => {
                let error = TransportError::BindFailed(e);
                tracing::error!(
                    error = %error,
                    "Failed to get local socket address"
                );
                return Err(error);
            }
        };

        tracing::info!(
            "BACnet/IP transport bound to {} with broadcast address {}",
            local_address,
            broadcast_address
        );

        Ok(Self {
            socket,
            broadcast_address,
            local_address,
        })
    }
}

impl Transport for BacnetIpTransport {
    fn send(&self, address: &Address, data: &[u8]) -> Result<(), TransportError> {
        match address {
            Address::Ip(socket_addr) => {
                // Wrap in BVLC Original-Unicast-NPDU header
                let len = 4 + data.len();
                let mut packet = Vec::with_capacity(len);
                packet.push(0x81);
                packet.push(0x0A); // Original-Unicast-NPDU
                packet.extend_from_slice(&(len as u16).to_be_bytes());
                packet.extend_from_slice(data);

                match self.socket.send_to(&packet, socket_addr) {
                    Ok(_) => {
                        tracing::debug!("Sent {} bytes to {}", data.len(), socket_addr);
                        Ok(())
                    }
                    Err(e) => {
                        let error = TransportError::SendFailed(e);
                        tracing::error!(
                            address = %socket_addr,
                            data_len = data.len(),
                            error = %error,
                            "Failed to send data to address"
                        );
                        Err(error)
                    }
                }
            }
            Address::MsTp { network, mac } => {
                let error = TransportError::SendFailed(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Cannot send MS/TP address over BACnet/IP transport",
                ));
                tracing::error!(
                    network = network,
                    mac = ?mac,
                    error = %error,
                    "Attempted to send to MS/TP address over BACnet/IP transport"
                );
                Err(error)
            }
        }
    }

    fn broadcast(&self, data: &[u8]) -> Result<(), TransportError> {
        // Wrap in BVLC Original-Broadcast-NPDU header
        let len = 4 + data.len();
        let mut packet = Vec::with_capacity(len);
        packet.push(0x81);
        packet.push(0x0B); // Original-Broadcast-NPDU
        packet.extend_from_slice(&(len as u16).to_be_bytes());
        packet.extend_from_slice(data);

        match self.socket.send_to(&packet, self.broadcast_address) {
            Ok(_) => {
                tracing::debug!(
                    "Broadcast {} bytes to {}",
                    data.len(),
                    self.broadcast_address
                );
                Ok(())
            }
            Err(e) => {
                let error = TransportError::SendFailed(e);
                tracing::error!(
                    broadcast_address = %self.broadcast_address,
                    data_len = data.len(),
                    error = %error,
                    "Failed to broadcast data"
                );
                Err(error)
            }
        }
    }

    fn receive(&self, timeout: Duration) -> Result<(Address, Vec<u8>), TransportError> {
        // Set socket read timeout
        if let Err(e) = self.socket.set_read_timeout(Some(timeout)) {
            let error = TransportError::ReceiveFailed(e);
            tracing::error!(
                timeout_ms = timeout.as_millis(),
                error = %error,
                "Failed to set socket read timeout"
            );
            return Err(error);
        }

        // Receive data into buffer
        let mut buffer = vec![0u8; 65535]; // Maximum UDP packet size
        match self.socket.recv_from(&mut buffer) {
            Ok((size, source_addr)) => {
                buffer.truncate(size);
                // Strip BVLC header if present (BACnet/IP)
                let payload = if buffer.len() >= 4 && buffer[0] == 0x81 && (buffer[1] == 0x0A || buffer[1] == 0x0B) {
                    buffer[4..].to_vec()
                } else {
                    buffer
                };
                tracing::debug!("Received {} bytes from {}", payload.len(), source_addr);
                Ok((Address::Ip(source_addr), payload))
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut =>
            {
                tracing::error!(
                    timeout_ms = timeout.as_millis(),
                    local_address = %self.local_address,
                    "Receive operation timed out"
                );
                Err(TransportError::Timeout)
            }
            Err(e) => {
                let error = TransportError::ReceiveFailed(e);
                tracing::error!(
                    timeout_ms = timeout.as_millis(),
                    local_address = %self.local_address,
                    error = %error,
                    "Failed to receive data"
                );
                Err(error)
            }
        }
    }

    fn local_address(&self) -> Address {
        Address::Ip(self.local_address)
    }
}

// Re-export MS/TP transport types
pub use bbmd::{BbmdConfig, BbmdTransport};
pub use mstp::{BacnetMstpTransport, TokenState};
pub use packet_log::{LoggedTransport, PacketDirection, PacketLog, PacketRecord};
pub use router::{RouteEntry, RouterTransport};
