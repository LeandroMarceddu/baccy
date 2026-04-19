// Network manager for transport selection and management

use crate::transport_config::TransportConfig;
use crate::AppError;
use baccy_protocol::BacnetService;
use baccy_transport::{BacnetIpTransport, BacnetMstpTransport, Transport};
use std::sync::Arc;
use std::time::Duration;

/// Manages BACnet network transport and service
pub struct NetworkManager {
    transport: Arc<dyn Transport>,
    config: TransportConfig,
    service: Arc<BacnetService>,
}

impl NetworkManager {
    /// Create a new NetworkManager with the specified transport configuration
    ///
    /// # Arguments
    /// * `config` - Transport configuration (BACnet/IP or MS/TP)
    /// * `timeout` - Timeout duration for BACnet operations
    ///
    /// # Returns
    /// A new `NetworkManager` instance or an error if transport initialization fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - BACnet/IP: Failed to bind to the specified address
    /// - MS/TP: Failed to open serial port, invalid baud rate, or permission denied
    pub fn new(config: TransportConfig, timeout: Duration) -> Result<Self, AppError> {
        let transport: Arc<dyn Transport> = match &config {
            TransportConfig::Ip { bind_addr } => {
                tracing::info!("Creating BACnet/IP transport on {}", bind_addr);
                Arc::new(
                    BacnetIpTransport::bind(*bind_addr)
                        .map_err(|e| AppError::ProtocolError(e.into()))?,
                )
            }
            TransportConfig::MsTp {
                port_name,
                baud_rate,
                local_mac,
            } => {
                tracing::info!(
                    "Creating MS/TP transport on {} @ {} bps, MAC {}",
                    port_name,
                    baud_rate,
                    local_mac
                );
                Arc::new(
                    BacnetMstpTransport::new(port_name, *baud_rate, *local_mac)
                        .map_err(|e| AppError::ProtocolError(e.into()))?,
                )
            }
        };

        let service = Arc::new(BacnetService::new(Arc::clone(&transport), timeout));

        tracing::info!("NetworkManager created with {}", config.description());

        Ok(Self {
            transport,
            config,
            service,
        })
    }

    /// Switch to a different transport configuration
    ///
    /// This will shut down the current transport and create a new one with the
    /// specified configuration. All existing device/object/property state will
    /// be lost.
    ///
    /// # Arguments
    /// * `config` - New transport configuration
    /// * `timeout` - Timeout duration for BACnet operations
    ///
    /// # Returns
    /// Ok(()) if the transport was successfully switched
    ///
    /// # Errors
    /// Returns an error if the new transport fails to initialize
    pub fn switch_transport(
        &mut self,
        config: TransportConfig,
        timeout: Duration,
    ) -> Result<(), AppError> {
        tracing::info!(
            "Switching transport from {} to {}",
            self.config.description(),
            config.description()
        );

        let transport: Arc<dyn Transport> = match &config {
            TransportConfig::Ip { bind_addr } => {
                Arc::new(
                    BacnetIpTransport::bind(*bind_addr)
                        .map_err(|e| AppError::ProtocolError(e.into()))?,
                )
            }
            TransportConfig::MsTp {
                port_name,
                baud_rate,
                local_mac,
            } => {
                Arc::new(
                    BacnetMstpTransport::new(port_name, *baud_rate, *local_mac)
                        .map_err(|e| AppError::ProtocolError(e.into()))?,
                )
            }
        };

        let service = Arc::new(BacnetService::new(Arc::clone(&transport), timeout));

        self.transport = transport;
        self.config = config;
        self.service = service;

        tracing::info!("Transport switched successfully");

        Ok(())
    }

    /// Get the current transport configuration
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// Get a reference to the BACnet service
    pub fn service(&self) -> &Arc<BacnetService> {
        &self.service
    }

    /// Get a reference to the transport
    pub fn transport(&self) -> &Arc<dyn Transport> {
        &self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn test_network_manager_ip_creation() {
        let addr: SocketAddr = "0.0.0.0:47808".parse().unwrap();
        let config = TransportConfig::new_ip(addr);
        let timeout = Duration::from_secs(5);

        let manager = NetworkManager::new(config.clone(), timeout);
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert_eq!(manager.config(), &config);
    }

    #[test]
    fn test_network_manager_config_description() {
        let addr: SocketAddr = "0.0.0.0:47808".parse().unwrap();
        let config = TransportConfig::new_ip(addr);
        let timeout = Duration::from_secs(5);

        let manager = NetworkManager::new(config, timeout).unwrap();
        let desc = manager.config().description();

        assert!(desc.contains("BACnet/IP"));
        assert!(desc.contains("0.0.0.0:47808"));
    }
}
