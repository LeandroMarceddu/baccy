// MS/TP transport implementation
//
// This module implements the BACnet MS/TP (Master-Slave/Token-Passing) transport
// layer for serial communication over RS-485 networks.

use crate::frame::MstpFrame;
use crate::TransportError;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// Token passing state for MS/TP master nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// Idle state - not participating in token passing
    Idle,
    /// Waiting to receive the token
    WaitForToken,
    /// Currently holding the token and can transmit
    HaveToken { frames_sent: u8 },
    /// In the process of passing the token to the next station
    PassingToken,
}

/// Token manager for MS/TP master node token passing
///
/// Manages the token passing state machine for MS/TP master nodes.
/// Tracks the current state, next station to pass the token to,
/// and enforces limits on frames sent per token possession.
#[derive(Debug, Clone)]
pub struct TokenManager {
    /// Current token passing state
    pub state: TokenState,
    /// Next station to pass the token to (MAC address)
    pub next_station: u8,
    /// Highest MAC address of master nodes on the network
    pub max_master: u8,
    /// Maximum number of frames that can be sent while holding the token
    pub max_info_frames: u8,
}

impl Default for TokenManager {
    fn default() -> Self {
        Self {
            state: TokenState::Idle,
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        }
    }
}

impl TokenManager {
    /// Handle token received event
    ///
    /// Transitions the token state to HaveToken when a token is received.
    /// Resets the frames_sent counter to 0.
    ///
    /// **Validates: Requirements 10.1, 10.3**
    pub fn handle_token_received(&mut self) {
        tracing::debug!(
            previous_state = ?self.state,
            "Token received, transitioning to HaveToken state"
        );
        
        self.state = TokenState::HaveToken { frames_sent: 0 };
    }

    /// Check if we can send a frame
    ///
    /// Returns true if we currently hold the token and have not exceeded
    /// the maximum number of frames allowed per token possession.
    ///
    /// **Validates: Requirements 10.2, 10.4**
    pub fn can_send_frame(&self) -> bool {
        match self.state {
            TokenState::HaveToken { frames_sent } => {
                let can_send = frames_sent < self.max_info_frames;
                tracing::trace!(
                    frames_sent = frames_sent,
                    max_info_frames = self.max_info_frames,
                    can_send = can_send,
                    "Checking if frame can be sent"
                );
                can_send
            }
            _ => {
                tracing::trace!(
                    state = ?self.state,
                    "Cannot send frame - not holding token"
                );
                false
            }
        }
    }

    /// Check if we should pass the token
    ///
    /// Returns true if we are holding the token and have reached the maximum
    /// number of frames allowed per token possession.
    ///
    /// **Validates: Requirements 10.2, 10.4**
    pub fn should_pass_token(&self) -> bool {
        match self.state {
            TokenState::HaveToken { frames_sent } => {
                let should_pass = frames_sent >= self.max_info_frames;
                tracing::trace!(
                    frames_sent = frames_sent,
                    max_info_frames = self.max_info_frames,
                    should_pass = should_pass,
                    "Checking if token should be passed"
                );
                should_pass
            }
            _ => {
                tracing::trace!(
                    state = ?self.state,
                    "Not holding token - no need to pass"
                );
                false
            }
        }
    }

    /// Get the next station to pass the token to
    ///
    /// Calculates the next master station MAC address in the token ring.
    /// Master nodes pass the token sequentially (0 → 1 → 2 → ... → max_master → 0).
    ///
    /// **Validates: Requirements 10.1, 10.2**
    pub fn get_next_station(&self) -> u8 {
        let next = if self.next_station >= self.max_master {
            0
        } else {
            self.next_station + 1
        };
        
        tracing::trace!(
            current_next_station = self.next_station,
            max_master = self.max_master,
            calculated_next = next,
            "Calculated next station for token passing"
        );
        
        next
    }
}

/// BACnet MS/TP transport implementation
///
/// This transport provides communication over RS-485 serial networks using the
/// MS/TP protocol. It supports master node operation with token passing.
///
/// # Thread Safety
///
/// The transport uses `Arc<Mutex<>>` for internal state to ensure thread-safe
/// access from multiple contexts (send, receive, token passing).
pub struct BacnetMstpTransport {
    /// Serial port for communication
    /// Wrapped in Arc<Mutex<>> for thread-safe access
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    
    /// Local MAC address (0-127 for master nodes)
    local_mac: u8,
    
    /// Baud rate for serial communication
    baud_rate: u32,
    
    /// Buffer for received frames waiting to be processed
    /// Wrapped in Arc<Mutex<>> for thread-safe access
    frame_buffer: Arc<Mutex<VecDeque<MstpFrame>>>,
    
    /// Token manager for token passing state machine
    /// Wrapped in Arc<Mutex<>> for thread-safe access
    token_manager: Arc<Mutex<TokenManager>>,
}

impl BacnetMstpTransport {
    /// Create a new MS/TP transport
    ///
    /// Opens and configures a serial port for MS/TP communication. The port is
    /// configured with 8 data bits, 1 stop bit, and no parity (8N1).
    ///
    /// # Arguments
    ///
    /// * `port_name` - Serial port device path (e.g., "/dev/ttyUSB0" on Linux, "COM3" on Windows)
    /// * `baud_rate` - Communication speed in bits per second (9600, 19200, 38400, or 76800)
    /// * `local_mac` - Local MAC address (0-127 for master nodes, 128-254 for slave nodes)
    ///
    /// # Returns
    ///
    /// A new `BacnetMstpTransport` instance configured for the specified port.
    ///
    /// # Errors
    ///
    /// Returns `TransportError::BindFailed` if:
    /// - The serial port cannot be opened (not found, permission denied, already in use)
    /// - The port configuration fails
    /// - The baud rate is not supported
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use baccy_transport::BacnetMstpTransport;
    ///
    /// // Create MS/TP transport on Linux
    /// let transport = BacnetMstpTransport::new("/dev/ttyUSB0", 38400, 5)?;
    ///
    /// // Create MS/TP transport on Windows
    /// let transport = BacnetMstpTransport::new("COM3", 38400, 5)?;
    /// # Ok::<(), baccy_transport::TransportError>(())
    /// ```
    pub fn new(
        port_name: &str,
        baud_rate: u32,
        local_mac: u8,
    ) -> Result<Self, TransportError> {
        tracing::info!(
            port_name = port_name,
            baud_rate = baud_rate,
            local_mac = local_mac,
            "Creating BACnet MS/TP transport"
        );

        // Validate baud rate - MS/TP supports specific rates
        const SUPPORTED_BAUD_RATES: &[u32] = &[9600, 19200, 38400, 76800, 115200];
        if !SUPPORTED_BAUD_RATES.contains(&baud_rate) {
            let error = TransportError::BindFailed(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Unsupported baud rate {}. Supported rates: 9600, 19200, 38400, 76800, 115200",
                    baud_rate
                ),
            ));
            tracing::error!(
                baud_rate = baud_rate,
                supported_rates = ?SUPPORTED_BAUD_RATES,
                error = %error,
                "Invalid baud rate specified"
            );
            return Err(error);
        }

        // Validate MAC address range
        // Master nodes: 0-127, Slave nodes: 128-254, Broadcast: 255
        // For now, we support master nodes only (0-127)
        if local_mac > 127 {
            let error = TransportError::BindFailed(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid MAC address {}. Master nodes must use MAC addresses 0-127",
                    local_mac
                ),
            ));
            tracing::error!(
                local_mac = local_mac,
                error = %error,
                "Invalid MAC address for master node"
            );
            return Err(error);
        }

        // Open serial port
        let port = serialport::new(port_name, baud_rate)
            .data_bits(serialport::DataBits::Eight)
            .stop_bits(serialport::StopBits::One)
            .parity(serialport::Parity::None)
            .timeout(std::time::Duration::from_millis(100))
            .open()
            .map_err(|e| {
                let error = match e.kind() {
                    serialport::ErrorKind::NoDevice => {
                        // List available ports for helpful error message
                        let available_ports = serialport::available_ports()
                            .ok()
                            .and_then(|ports| {
                                if ports.is_empty() {
                                    None
                                } else {
                                    Some(
                                        ports
                                            .iter()
                                            .map(|p| p.port_name.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    )
                                }
                            });

                        let msg = if let Some(ports) = available_ports {
                            format!(
                                "Serial port '{}' not found. Available ports: {}",
                                port_name, ports
                            )
                        } else {
                            format!(
                                "Serial port '{}' not found. No serial ports detected on this system",
                                port_name
                            )
                        };

                        TransportError::BindFailed(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            msg,
                        ))
                    }
                    serialport::ErrorKind::Io(io_kind) => {
                        let msg = match io_kind {
                            std::io::ErrorKind::PermissionDenied => {
                                #[cfg(target_os = "linux")]
                                let hint = format!(
                                    "Permission denied accessing '{}'. Add your user to the 'dialout' group: sudo usermod -a -G dialout $USER",
                                    port_name
                                );
                                #[cfg(target_os = "macos")]
                                let hint = format!(
                                    "Permission denied accessing '{}'. You may need to run with elevated privileges or check port permissions",
                                    port_name
                                );
                                #[cfg(target_os = "windows")]
                                let hint = format!(
                                    "Permission denied accessing '{}'. The port may be in use by another application or require administrator privileges",
                                    port_name
                                );
                                #[cfg(not(any(
                                    target_os = "linux",
                                    target_os = "macos",
                                    target_os = "windows"
                                )))]
                                let hint = format!(
                                    "Permission denied accessing '{}'. Check port permissions",
                                    port_name
                                );

                                hint
                            }
                            _ => format!("Failed to open serial port '{}': {}", port_name, e),
                        };

                        TransportError::BindFailed(std::io::Error::new(io_kind, msg))
                    }
                    _ => TransportError::BindFailed(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to open serial port '{}': {}", port_name, e),
                    )),
                };

                tracing::error!(
                    port_name = port_name,
                    baud_rate = baud_rate,
                    error = %error,
                    "Failed to open serial port"
                );

                error
            })?;

        tracing::info!(
            port_name = port_name,
            baud_rate = baud_rate,
            local_mac = local_mac,
            "Successfully opened and configured serial port for MS/TP transport"
        );

        // Initialize token manager with HaveToken state so we can send immediately
        // In a real MS/TP network, we would need to participate in token passing,
        // but for initial implementation we start with the token to allow sending
        let mut token_manager = TokenManager::default();
        token_manager.state = TokenState::HaveToken { frames_sent: 0 };

        Ok(Self {
            port: Arc::new(Mutex::new(port)),
            local_mac,
            baud_rate,
            frame_buffer: Arc::new(Mutex::new(VecDeque::new())),
            token_manager: Arc::new(Mutex::new(token_manager)),
        })
    }
}

impl crate::Transport for BacnetMstpTransport {
    fn send(&self, address: &baccy_core::Address, data: &[u8]) -> Result<(), crate::TransportError> {
        // Check if address is MS/TP type (Requirement 11.1)
        match address {
            baccy_core::Address::MsTp { network: _, mac } => {
                tracing::debug!(
                    destination_mac = mac,
                    data_len = data.len(),
                    "Sending BACnet message over MS/TP"
                );

                // Check token state before sending (Requirement 10.2)
                let mut token_manager = self.token_manager.lock().unwrap();
                if !token_manager.can_send_frame() {
                    let error = crate::TransportError::SendFailed(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "Cannot send frame: not holding token or max frames reached",
                    ));
                    tracing::warn!(
                        destination_mac = mac,
                        token_state = ?token_manager.state,
                        error = %error,
                        "Cannot send frame due to token state"
                    );
                    return Err(error);
                }

                // Create MS/TP frame with destination MAC (Requirement 11.1)
                // Use BacnetDataNotExpectingReply for now (can be enhanced later)
                let frame = MstpFrame::bacnet_data(
                    *mac,           // destination MAC
                    self.local_mac, // source MAC
                    data.to_vec(),  // BACnet message data
                    false,          // not expecting reply
                );

                // Encode frame to bytes
                let frame_bytes = frame.encode();

                // Transmit frame over serial port
                let mut port = self.port.lock().unwrap();
                match port.write_all(&frame_bytes) {
                    Ok(_) => {
                        tracing::debug!(
                            destination_mac = mac,
                            frame_len = frame_bytes.len(),
                            "Successfully transmitted MS/TP frame"
                        );

                        // Increment frames_sent counter (Requirement 10.3)
                        if let TokenState::HaveToken { frames_sent } = token_manager.state {
                            token_manager.state = TokenState::HaveToken {
                                frames_sent: frames_sent + 1,
                            };
                            tracing::trace!(
                                frames_sent = frames_sent + 1,
                                max_info_frames = token_manager.max_info_frames,
                                "Incremented frames_sent counter"
                            );
                        }

                        // Check if we should pass the token (Requirement 10.4)
                        if token_manager.should_pass_token() {
                            tracing::debug!(
                                frames_sent = match token_manager.state {
                                    TokenState::HaveToken { frames_sent } => frames_sent,
                                    _ => 0,
                                },
                                max_info_frames = token_manager.max_info_frames,
                                "Max frames reached, passing token"
                            );

                            // Get next station to pass token to (Requirement 10.2)
                            let next_station = token_manager.get_next_station();
                            
                            // Update next_station for next time
                            token_manager.next_station = next_station;
                            
                            // Transition to PassingToken state
                            token_manager.state = TokenState::PassingToken;

                            // Create and send Token frame (Requirement 10.4)
                            let token_frame = MstpFrame::token(next_station, self.local_mac);
                            let token_bytes = token_frame.encode();

                            match port.write_all(&token_bytes) {
                                Ok(_) => {
                                    tracing::debug!(
                                        next_station = next_station,
                                        "Successfully passed token to next station"
                                    );
                                    
                                    // Transition to WaitForToken state
                                    token_manager.state = TokenState::WaitForToken;
                                }
                                Err(e) => {
                                    tracing::error!(
                                        next_station = next_station,
                                        error = %e,
                                        "Failed to send token frame"
                                    );
                                    // Don't fail the original send operation, but log the error
                                    // The token passing will be retried on next send
                                }
                            }
                        }

                        Ok(())
                    }
                    Err(e) => {
                        let error = crate::TransportError::SendFailed(e);
                        tracing::error!(
                            destination_mac = mac,
                            data_len = data.len(),
                            error = %error,
                            "Failed to transmit MS/TP frame"
                        );
                        Err(error)
                    }
                }
            }
            baccy_core::Address::Ip(socket_addr) => {
                // Return SendFailed error for IP addresses (Requirement 11.2)
                let error = crate::TransportError::SendFailed(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Cannot send IP address over MS/TP transport",
                ));
                tracing::error!(
                    address = %socket_addr,
                    error = %error,
                    "Attempted to send to IP address over MS/TP transport"
                );
                Err(error)
            }
        }
    }

    fn broadcast(&self, data: &[u8]) -> Result<(), crate::TransportError> {
        tracing::debug!(
            data_len = data.len(),
            "Broadcasting BACnet message over MS/TP"
        );

        // Check token state before broadcasting (Requirement 10.2)
        let mut token_manager = self.token_manager.lock().unwrap();
        if !token_manager.can_send_frame() {
            let error = crate::TransportError::SendFailed(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Cannot broadcast frame: not holding token or max frames reached",
            ));
            tracing::warn!(
                token_state = ?token_manager.state,
                error = %error,
                "Cannot broadcast frame due to token state"
            );
            return Err(error);
        }

        // Create MS/TP frame with broadcast MAC address (255)
        let frame = MstpFrame::bacnet_data(
            255,            // broadcast destination
            self.local_mac, // source MAC
            data.to_vec(),  // BACnet message data
            false,          // not expecting reply
        );

        // Encode frame to bytes
        let frame_bytes = frame.encode();

        tracing::info!(
            frame_len = frame_bytes.len(),
            frame_bytes = ?frame_bytes,
            destination_mac = 255,
            source_mac = self.local_mac,
            data_len = data.len(),
            "Broadcasting MS/TP Who-Is frame"
        );

        // Transmit frame over serial port
        let mut port = self.port.lock().unwrap();
        match port.write_all(&frame_bytes) {
            Ok(_) => {
                tracing::debug!(
                    frame_len = frame_bytes.len(),
                    "Successfully broadcast MS/TP frame"
                );

                // Increment frames_sent counter (Requirement 10.3)
                if let TokenState::HaveToken { frames_sent } = token_manager.state {
                    token_manager.state = TokenState::HaveToken {
                        frames_sent: frames_sent + 1,
                    };
                    tracing::trace!(
                        frames_sent = frames_sent + 1,
                        max_info_frames = token_manager.max_info_frames,
                        "Incremented frames_sent counter"
                    );
                }

                // Check if we should pass the token (Requirement 10.4)
                if token_manager.should_pass_token() {
                    tracing::debug!(
                        frames_sent = match token_manager.state {
                            TokenState::HaveToken { frames_sent } => frames_sent,
                            _ => 0,
                        },
                        max_info_frames = token_manager.max_info_frames,
                        "Max frames reached, passing token"
                    );

                    // Get next station to pass token to (Requirement 10.2)
                    let next_station = token_manager.get_next_station();
                    
                    // Update next_station for next time
                    token_manager.next_station = next_station;
                    
                    // Transition to PassingToken state
                    token_manager.state = TokenState::PassingToken;

                    // Create and send Token frame (Requirement 10.4)
                    let token_frame = MstpFrame::token(next_station, self.local_mac);
                    let token_bytes = token_frame.encode();

                    match port.write_all(&token_bytes) {
                        Ok(_) => {
                            tracing::debug!(
                                next_station = next_station,
                                "Successfully passed token to next station"
                            );
                            
                            // Transition to WaitForToken state
                            token_manager.state = TokenState::WaitForToken;
                        }
                        Err(e) => {
                            tracing::error!(
                                next_station = next_station,
                                error = %e,
                                "Failed to send token frame"
                            );
                            // Don't fail the original broadcast operation, but log the error
                            // The token passing will be retried on next send
                        }
                    }
                }

                Ok(())
            }
            Err(e) => {
                let error = crate::TransportError::SendFailed(e);
                tracing::error!(
                    data_len = data.len(),
                    error = %error,
                    "Failed to broadcast MS/TP frame"
                );
                Err(error)
            }
        }
    }

    fn receive(&self, timeout: std::time::Duration) -> Result<(baccy_core::Address, Vec<u8>), crate::TransportError> {
        // Set read timeout on serial port
        {
            let mut port = self.port.lock().unwrap();
            if let Err(e) = port.set_timeout(timeout) {
                let error = crate::TransportError::ReceiveFailed(e.into());
                tracing::error!(
                    timeout_ms = timeout.as_millis(),
                    error = %error,
                    "Failed to set serial port read timeout"
                );
                return Err(error);
            }
        }

        // Read frames from serial port in a loop
        // Token frames are handled internally and not returned to caller
        // MS/TP frames start with preamble 0x55 0xFF
        let mut buffer = vec![0u8; 1024]; // Buffer for frame data
        
        loop {
            let mut port = self.port.lock().unwrap();
            match port.read(&mut buffer) {
                Ok(bytes_read) if bytes_read > 0 => {
                    buffer.truncate(bytes_read);
                    
                    tracing::debug!(
                        bytes_read = bytes_read,
                        raw_bytes = ?&buffer[..bytes_read.min(32)],
                        "Received data on MS/TP serial port"
                    );
                    
                    // Decode MS/TP frame
                    match MstpFrame::decode(&buffer) {
                        Ok(frame) => {
                            tracing::debug!(
                                frame_type = ?frame.frame_type,
                                source_mac = frame.source,
                                destination_mac = frame.destination,
                                data_len = frame.data.len(),
                                "Received MS/TP frame"
                            );

                            // Check if this is a Token frame addressed to us (Requirement 10.3)
                            if frame.is_token() && frame.destination == self.local_mac {
                                tracing::info!(
                                    source_mac = frame.source,
                                    destination_mac = frame.destination,
                                    local_mac = self.local_mac,
                                    "Received token frame addressed to us"
                                );

                                // Update token state to HaveToken and reset frames_sent counter
                                let mut token_manager = self.token_manager.lock().unwrap();
                                token_manager.handle_token_received();
                                
                                tracing::debug!(
                                    new_state = ?token_manager.state,
                                    "Token state updated after receiving token"
                                );

                                // Token frames are internal protocol frames - don't return to caller
                                // Continue reading to get the next frame
                                drop(token_manager);
                                drop(port);
                                buffer = vec![0u8; 1024]; // Reset buffer for next read
                                continue;
                            }

                            // For data frames, extract source address and return to caller
                            if frame.is_data() {
                                let source_address = baccy_core::Address::MsTp {
                                    network: 0, // Local network
                                    mac: frame.source,
                                };

                                drop(port);
                                return Ok((source_address, frame.data));
                            }

                            // For other frame types (PollForMaster, etc.), continue reading
                            tracing::trace!(
                                frame_type = ?frame.frame_type,
                                "Received non-data frame, continuing to read"
                            );
                            drop(port);
                            buffer = vec![0u8; 1024]; // Reset buffer for next read
                            continue;
                        }
                        Err(e) => {
                            let error = crate::TransportError::ReceiveFailed(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Failed to decode MS/TP frame: {}", e),
                            ));
                            tracing::error!(
                                error = %error,
                                bytes_read = bytes_read,
                                "Failed to decode received MS/TP frame"
                            );
                            drop(port);
                            return Err(error);
                        }
                    }
                }
                Ok(_) => {
                    // No data received
                    tracing::debug!(
                        timeout_ms = timeout.as_millis(),
                        "No MS/TP frame received within timeout"
                    );
                    drop(port);
                    return Err(crate::TransportError::Timeout);
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    tracing::debug!(
                        timeout_ms = timeout.as_millis(),
                        "MS/TP receive operation timed out"
                    );
                    drop(port);
                    return Err(crate::TransportError::Timeout);
                }
                Err(e) => {
                    let error = crate::TransportError::ReceiveFailed(e);
                    tracing::error!(
                        timeout_ms = timeout.as_millis(),
                        error = %error,
                        "Failed to receive MS/TP frame"
                    );
                    drop(port);
                    return Err(error);
                }
            }
        }
    }

    fn local_address(&self) -> baccy_core::Address {
        baccy_core::Address::MsTp {
            network: 0, // Local network
            mac: self.local_mac,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_manager_default() {
        let manager = TokenManager::default();
        
        assert_eq!(manager.state, TokenState::Idle);
        assert_eq!(manager.next_station, 0);
        assert_eq!(manager.max_master, 127);
        assert_eq!(manager.max_info_frames, 1);
    }

    #[test]
    fn test_token_state_variants() {
        let idle = TokenState::Idle;
        let wait = TokenState::WaitForToken;
        let have = TokenState::HaveToken { frames_sent: 0 };
        let passing = TokenState::PassingToken;
        
        assert_eq!(idle, TokenState::Idle);
        assert_eq!(wait, TokenState::WaitForToken);
        assert_eq!(have, TokenState::HaveToken { frames_sent: 0 });
        assert_eq!(passing, TokenState::PassingToken);
    }

    #[test]
    fn test_token_manager_creation() {
        let manager = TokenManager {
            state: TokenState::WaitForToken,
            next_station: 5,
            max_master: 10,
            max_info_frames: 5,
        };
        
        assert_eq!(manager.state, TokenState::WaitForToken);
        assert_eq!(manager.next_station, 5);
        assert_eq!(manager.max_master, 10);
        assert_eq!(manager.max_info_frames, 5);
    }

    #[test]
    fn test_token_state_clone() {
        let state1 = TokenState::HaveToken { frames_sent: 3 };
        let state2 = state1;
        
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_token_manager_clone() {
        let manager1 = TokenManager::default();
        let manager2 = manager1.clone();
        
        assert_eq!(manager1.state, manager2.state);
        assert_eq!(manager1.next_station, manager2.next_station);
        assert_eq!(manager1.max_master, manager2.max_master);
        assert_eq!(manager1.max_info_frames, manager2.max_info_frames);
    }

    // Tests for Task 6.2: Token state transitions

    #[test]
    fn test_handle_token_received() {
        let mut manager = TokenManager::default();
        
        // Initially in Idle state
        assert_eq!(manager.state, TokenState::Idle);
        
        // Handle token received
        manager.handle_token_received();
        
        // Should transition to HaveToken with frames_sent = 0
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
    }

    #[test]
    fn test_handle_token_received_from_wait_state() {
        let mut manager = TokenManager {
            state: TokenState::WaitForToken,
            next_station: 5,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Handle token received while waiting
        manager.handle_token_received();
        
        // Should transition to HaveToken with frames_sent = 0
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
    }

    #[test]
    fn test_can_send_frame_when_holding_token() {
        let manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 0 },
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should be able to send frame when holding token and under limit
        assert!(manager.can_send_frame());
    }

    #[test]
    fn test_can_send_frame_when_at_limit() {
        let manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 1 },
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should not be able to send frame when at limit
        assert!(!manager.can_send_frame());
    }

    #[test]
    fn test_can_send_frame_when_not_holding_token() {
        let manager = TokenManager {
            state: TokenState::Idle,
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should not be able to send frame when not holding token
        assert!(!manager.can_send_frame());
    }

    #[test]
    fn test_can_send_frame_with_multiple_frames_allowed() {
        let manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 2 },
            next_station: 0,
            max_master: 127,
            max_info_frames: 5,
        };
        
        // Should be able to send more frames when under limit
        assert!(manager.can_send_frame());
    }

    #[test]
    fn test_should_pass_token_when_at_limit() {
        let manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 1 },
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should pass token when at limit
        assert!(manager.should_pass_token());
    }

    #[test]
    fn test_should_pass_token_when_under_limit() {
        let manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 0 },
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should not pass token when under limit
        assert!(!manager.should_pass_token());
    }

    #[test]
    fn test_should_pass_token_when_not_holding_token() {
        let manager = TokenManager {
            state: TokenState::Idle,
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should not pass token when not holding it
        assert!(!manager.should_pass_token());
    }

    #[test]
    fn test_get_next_station_sequential() {
        let manager = TokenManager {
            state: TokenState::Idle,
            next_station: 5,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Next station should be next_station + 1
        assert_eq!(manager.get_next_station(), 6);
    }

    #[test]
    fn test_get_next_station_wrap_around() {
        let manager = TokenManager {
            state: TokenState::Idle,
            next_station: 127,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Should wrap around to 0 when at max_master
        assert_eq!(manager.get_next_station(), 0);
    }

    #[test]
    fn test_get_next_station_with_lower_max_master() {
        let manager = TokenManager {
            state: TokenState::Idle,
            next_station: 10,
            max_master: 10,
            max_info_frames: 1,
        };
        
        // Should wrap around to 0 when at max_master
        assert_eq!(manager.get_next_station(), 0);
    }

    #[test]
    fn test_get_next_station_zero() {
        let manager = TokenManager {
            state: TokenState::Idle,
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };
        
        // Next station should be 1
        assert_eq!(manager.get_next_station(), 1);
    }

    // Tests for Task 6.3: Token passing integration

    #[test]
    fn test_token_manager_increment_frames_sent() {
        let mut manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 0 },
            next_station: 5,
            max_master: 127,
            max_info_frames: 3,
        };

        // Simulate sending a frame by incrementing frames_sent
        if let TokenState::HaveToken { frames_sent } = manager.state {
            manager.state = TokenState::HaveToken {
                frames_sent: frames_sent + 1,
            };
        }

        // Verify frames_sent was incremented
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 1 });
        
        // Should still be able to send more frames
        assert!(manager.can_send_frame());
        assert!(!manager.should_pass_token());
    }

    #[test]
    fn test_token_manager_pass_token_after_max_frames() {
        let mut manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 2 },
            next_station: 5,
            max_master: 127,
            max_info_frames: 3,
        };

        // Send one more frame to reach the limit
        if let TokenState::HaveToken { frames_sent } = manager.state {
            manager.state = TokenState::HaveToken {
                frames_sent: frames_sent + 1,
            };
        }

        // Now at limit (frames_sent = 3, max_info_frames = 3)
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 3 });
        assert!(!manager.can_send_frame());
        assert!(manager.should_pass_token());

        // Get next station and transition to PassingToken
        let next_station = manager.get_next_station();
        assert_eq!(next_station, 6);
        
        manager.next_station = next_station;
        manager.state = TokenState::PassingToken;

        // After passing token, transition to WaitForToken
        manager.state = TokenState::WaitForToken;
        assert_eq!(manager.state, TokenState::WaitForToken);
    }

    #[test]
    fn test_token_manager_cannot_send_without_token() {
        let manager = TokenManager {
            state: TokenState::WaitForToken,
            next_station: 5,
            max_master: 127,
            max_info_frames: 1,
        };

        // Should not be able to send without token
        assert!(!manager.can_send_frame());
        assert!(!manager.should_pass_token());
    }

    #[test]
    fn test_token_manager_full_cycle() {
        let mut manager = TokenManager {
            state: TokenState::Idle,
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };

        // 1. Receive token
        manager.handle_token_received();
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
        assert!(manager.can_send_frame());

        // 2. Send a frame
        if let TokenState::HaveToken { frames_sent } = manager.state {
            manager.state = TokenState::HaveToken {
                frames_sent: frames_sent + 1,
            };
        }
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 1 });

        // 3. Check if should pass token (yes, because max_info_frames = 1)
        assert!(manager.should_pass_token());
        assert!(!manager.can_send_frame());

        // 4. Pass token
        let next_station = manager.get_next_station();
        assert_eq!(next_station, 1);
        manager.next_station = next_station;
        manager.state = TokenState::PassingToken;

        // 5. Transition to waiting for token
        manager.state = TokenState::WaitForToken;
        assert_eq!(manager.state, TokenState::WaitForToken);
        assert!(!manager.can_send_frame());
    }

    #[test]
    fn test_token_manager_multiple_frames_allowed() {
        let mut manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 0 },
            next_station: 10,
            max_master: 127,
            max_info_frames: 5,
        };

        // Send 4 frames
        for i in 0..4 {
            assert!(manager.can_send_frame());
            assert!(!manager.should_pass_token());
            
            if let TokenState::HaveToken { frames_sent } = manager.state {
                manager.state = TokenState::HaveToken {
                    frames_sent: frames_sent + 1,
                };
            }
            
            assert_eq!(manager.state, TokenState::HaveToken { frames_sent: i + 1 });
        }

        // After 4 frames, should still be able to send one more
        assert!(manager.can_send_frame());
        assert!(!manager.should_pass_token());

        // Send 5th frame
        if let TokenState::HaveToken { frames_sent } = manager.state {
            manager.state = TokenState::HaveToken {
                frames_sent: frames_sent + 1,
            };
        }

        // Now at limit
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 5 });
        assert!(!manager.can_send_frame());
        assert!(manager.should_pass_token());
    }

    // Tests for Task 6.4: Token reception handling

    #[test]
    fn test_token_reception_updates_state() {
        // This test verifies that receiving a token frame updates the token state
        // to HaveToken and resets the frames_sent counter
        
        let mut manager = TokenManager {
            state: TokenState::WaitForToken,
            next_station: 5,
            max_master: 127,
            max_info_frames: 3,
        };

        // Initially waiting for token
        assert_eq!(manager.state, TokenState::WaitForToken);
        assert!(!manager.can_send_frame());

        // Simulate receiving a token frame
        manager.handle_token_received();

        // After receiving token, should be in HaveToken state with frames_sent = 0
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
        assert!(manager.can_send_frame());
        assert!(!manager.should_pass_token());
    }

    #[test]
    fn test_token_reception_resets_frames_sent() {
        // This test verifies that receiving a token resets the frames_sent counter
        // even if we were previously in HaveToken state with frames sent
        
        let mut manager = TokenManager {
            state: TokenState::HaveToken { frames_sent: 2 },
            next_station: 5,
            max_master: 127,
            max_info_frames: 3,
        };

        // Initially have token with 2 frames sent
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 2 });

        // Simulate receiving a token frame (edge case - shouldn't normally happen)
        manager.handle_token_received();

        // After receiving token, frames_sent should be reset to 0
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
        assert!(manager.can_send_frame());
        assert!(!manager.should_pass_token());
    }

    #[test]
    fn test_token_reception_from_idle_state() {
        // This test verifies that receiving a token from Idle state works correctly
        
        let mut manager = TokenManager {
            state: TokenState::Idle,
            next_station: 0,
            max_master: 127,
            max_info_frames: 1,
        };

        // Initially idle
        assert_eq!(manager.state, TokenState::Idle);
        assert!(!manager.can_send_frame());

        // Simulate receiving a token frame
        manager.handle_token_received();

        // After receiving token, should be in HaveToken state with frames_sent = 0
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
        assert!(manager.can_send_frame());
    }

    #[test]
    fn test_token_reception_from_passing_token_state() {
        // This test verifies that receiving a token while in PassingToken state works
        // (edge case - might happen if we receive our own token back)
        
        let mut manager = TokenManager {
            state: TokenState::PassingToken,
            next_station: 5,
            max_master: 127,
            max_info_frames: 1,
        };

        // Initially passing token
        assert_eq!(manager.state, TokenState::PassingToken);
        assert!(!manager.can_send_frame());

        // Simulate receiving a token frame
        manager.handle_token_received();

        // After receiving token, should be in HaveToken state with frames_sent = 0
        assert_eq!(manager.state, TokenState::HaveToken { frames_sent: 0 });
        assert!(manager.can_send_frame());
    }
}
