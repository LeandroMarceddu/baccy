// Shared application state for Tauri

use baccy_app::{CovManager, DeviceManager, ObjectManager, PropertyManager, TransportConfig, TrendingManager};
use baccy_protocol::{BacnetService, RetryConfig, ThrottleConfig};
use baccy_transport::network_stats::StatsCollector;
use baccy_transport::packet_log::{LoggedTransport, PacketLog};
use baccy_transport::{BacnetIpTransport, BacnetMstpTransport, BbmdConfig, BbmdTransport, Transport};
use crate::commands::write_prefs::WriteProtection;
use baccy_transport::bbmd::ForeignDeviceEntry;
use baccy_transport::router::RouterTransport;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Application settings for persistence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    /// Last used transport configuration
    pub last_transport: Option<TransportConfig>,
}

/// Shared application state
pub struct AppState {
    pub device_manager: Arc<Mutex<Option<DeviceManager>>>,
    pub object_manager: Arc<Mutex<Option<ObjectManager>>>,
    pub property_manager: Arc<Mutex<Option<PropertyManager>>>,
    pub trending_manager: Arc<Mutex<Option<TrendingManager>>>,
    pub cov_manager: Arc<Mutex<Option<CovManager>>>,
    pub service: Arc<Mutex<Option<Arc<BacnetService>>>>,
    pub packet_log: Arc<Mutex<Option<Arc<PacketLog>>>>,
    pub stats: Arc<StatsCollector>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub write_protection: WriteProtection,
    pub router: Arc<Mutex<RouterTransport>>,
    pub bbmd_state: Arc<Mutex<BbmdState>>,
}

/// Tracks BBMD state observable from the Tauri layer
pub struct BbmdState {
    pub enabled: bool,
    pub bbmd_address: Option<std::net::SocketAddr>,
    pub last_registration: Option<Instant>,
    pub ttl: Option<u32>,
    pub fdt_entries: Vec<ForeignDeviceEntry>,
}

impl BbmdState {
    pub fn new() -> Self {
        Self {
            enabled: false,
            bbmd_address: None,
            last_registration: None,
            ttl: None,
            fdt_entries: Vec::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            device_manager: Arc::new(Mutex::new(None)),
            object_manager: Arc::new(Mutex::new(None)),
            property_manager: Arc::new(Mutex::new(None)),
            trending_manager: Arc::new(Mutex::new(None)),
            cov_manager: Arc::new(Mutex::new(None)),
            service: Arc::new(Mutex::new(None)),
            packet_log: Arc::new(Mutex::new(None)),
            stats: Arc::new(StatsCollector::new()),
            settings: Arc::new(Mutex::new(AppSettings::default())),
            write_protection: WriteProtection::new(),
            router: Arc::new(Mutex::new(RouterTransport::new())),
            bbmd_state: Arc::new(Mutex::new(BbmdState::new())),
        }
    }

    /// Initialize the BACnet service with the selected network interface
    pub fn initialize_service(&self, ip: std::net::Ipv4Addr, port: u16, timeout_ms: u64) -> Result<(), String> {
        let bind_addr = std::net::SocketAddr::new(std::net::IpAddr::V4(ip), port);

        let packet_log = Arc::new(PacketLog::new(1000));
        let transport = BacnetIpTransport::bind(bind_addr)
            .map_err(|e| format!("Failed to bind to {}: {}", bind_addr, e))?;
        let logged_transport = Arc::new(LoggedTransport::new(Arc::new(transport), Arc::clone(&packet_log)));

        let timeout = Duration::from_millis(timeout_ms);
        let service = Arc::new(BacnetService::with_config(
            logged_transport as Arc<dyn Transport>,
            timeout,
            Arc::clone(&self.stats),
            RetryConfig::default(),
            ThrottleConfig::default(),
        ));

        // Initialize managers
        let device_manager = DeviceManager::new(Arc::clone(&service));
        let object_manager = ObjectManager::new(Arc::clone(&service));
        let property_manager = PropertyManager::new(Arc::clone(&service));
        let trending_manager = TrendingManager::new(Arc::clone(&service));
        let cov_manager = CovManager::new(Arc::clone(&service));

        // Store in state
        *self.service.lock().unwrap() = Some(service);
        *self.device_manager.lock().unwrap() = Some(device_manager);
        *self.object_manager.lock().unwrap() = Some(object_manager);
        *self.property_manager.lock().unwrap() = Some(property_manager);
        *self.trending_manager.lock().unwrap() = Some(trending_manager);
        *self.cov_manager.lock().unwrap() = Some(cov_manager);
        *self.packet_log.lock().unwrap() = Some(Arc::clone(&packet_log));

        // Save transport configuration
        let config = TransportConfig::new_ip(bind_addr);
        self.settings.lock().unwrap().last_transport = Some(config);

        tracing::info!("BACnet service initialized on {}", bind_addr);
        Ok(())
    }

    /// Initialize the BACnet service with BBMD-enabled transport
    pub fn initialize_bbmd_service(
        &self,
        ip: std::net::Ipv4Addr,
        port: u16,
        timeout_ms: u64,
        bbmd_enabled: bool,
        bbmd_address: Option<std::net::SocketAddr>,
        registration_ttl: u32,
    ) -> Result<(), String> {
        let bind_addr = std::net::SocketAddr::new(std::net::IpAddr::V4(ip), port);

        let packet_log = Arc::new(PacketLog::new(1000));
        let config = BbmdConfig {
            enabled: bbmd_enabled,
            register_with_bbmd: bbmd_address,
            registration_ttl,
        };

        let transport = BbmdTransport::bind(bind_addr, config)
            .map_err(|e| format!("Failed to bind BBMD transport to {}: {}", bind_addr, e))?;
        let logged_transport = Arc::new(LoggedTransport::new(Arc::new(transport), Arc::clone(&packet_log)));

        let timeout = Duration::from_millis(timeout_ms);
        let service = Arc::new(BacnetService::with_config(
            logged_transport as Arc<dyn Transport>,
            timeout,
            Arc::clone(&self.stats),
            RetryConfig::default(),
            ThrottleConfig::default(),
        ));

        // Initialize managers
        let device_manager = DeviceManager::new(Arc::clone(&service));
        let object_manager = ObjectManager::new(Arc::clone(&service));
        let property_manager = PropertyManager::new(Arc::clone(&service));
        let trending_manager = TrendingManager::new(Arc::clone(&service));
        let cov_manager = CovManager::new(Arc::clone(&service));

        // Store in state
        *self.service.lock().unwrap() = Some(service);
        *self.device_manager.lock().unwrap() = Some(device_manager);
        *self.object_manager.lock().unwrap() = Some(object_manager);
        *self.property_manager.lock().unwrap() = Some(property_manager);
        *self.trending_manager.lock().unwrap() = Some(trending_manager);
        *self.cov_manager.lock().unwrap() = Some(cov_manager);
        *self.packet_log.lock().unwrap() = Some(Arc::clone(&packet_log));

        // Save transport configuration
        let config = TransportConfig::new_ip(bind_addr);
        self.settings.lock().unwrap().last_transport = Some(config);

        tracing::info!(
            "BBMD service initialized on {} (enabled: {}, register: {:?})",
            bind_addr,
            bbmd_enabled,
            bbmd_address,
        );
        Ok(())
    }

    /// Initialize the BACnet service with MS/TP transport
    pub fn initialize_mstp_service(
        &self,
        port_name: String,
        baud_rate: u32,
        local_mac: u8,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let packet_log = Arc::new(PacketLog::new(1000));
        let transport = BacnetMstpTransport::new(&port_name, baud_rate, local_mac)
            .map_err(|e| Self::format_mstp_error(&e, &port_name))?;
        let logged_transport = Arc::new(LoggedTransport::new(Arc::new(transport), Arc::clone(&packet_log)));

        let timeout = Duration::from_millis(timeout_ms);
        let service = Arc::new(BacnetService::with_config(
            logged_transport as Arc<dyn Transport>,
            timeout,
            Arc::clone(&self.stats),
            RetryConfig::default(),
            ThrottleConfig::default(),
        ));

        // Initialize managers
        let device_manager = DeviceManager::new(Arc::clone(&service));
        let object_manager = ObjectManager::new(Arc::clone(&service));
        let property_manager = PropertyManager::new(Arc::clone(&service));
        let trending_manager = TrendingManager::new(Arc::clone(&service));
        let cov_manager = CovManager::new(Arc::clone(&service));

        // Store in state
        *self.service.lock().unwrap() = Some(service);
        *self.device_manager.lock().unwrap() = Some(device_manager);
        *self.object_manager.lock().unwrap() = Some(object_manager);
        *self.property_manager.lock().unwrap() = Some(property_manager);
        *self.trending_manager.lock().unwrap() = Some(trending_manager);
        *self.cov_manager.lock().unwrap() = Some(cov_manager);
        *self.packet_log.lock().unwrap() = Some(Arc::clone(&packet_log));

        // Save transport configuration
        let config = TransportConfig::new_mstp(port_name.clone(), baud_rate, local_mac);
        self.settings.lock().unwrap().last_transport = Some(config);

        tracing::info!(
            "MS/TP service initialized on {} @ {} bps, MAC {}",
            port_name,
            baud_rate,
            local_mac
        );
        Ok(())
    }

    /// Format MS/TP error messages with helpful hints
    fn format_mstp_error(error: &baccy_transport::TransportError, port_name: &str) -> String {
        let error_str = error.to_string();
        
        // Permission denied error
        if error_str.contains("Permission denied") || error_str.contains("Access is denied") {
            return format!(
                "Cannot access serial port '{}': Permission denied.\n\n\
                On Linux/macOS: Add your user to the 'dialout' group:\n\
                  sudo usermod -a -G dialout $USER\n\
                  (then log out and log back in)\n\n\
                On Windows: Run the application as Administrator or check device permissions.",
                port_name
            );
        }
        
        // Port not found error
        if error_str.contains("not found") || error_str.contains("No such file") {
            let available_ports = serialport::available_ports()
                .map(|ports| {
                    if ports.is_empty() {
                        "No serial ports found".to_string()
                    } else {
                        ports
                            .iter()
                            .map(|p| p.port_name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                })
                .unwrap_or_else(|_| "Unable to list ports".to_string());
            
            return format!(
                "Serial port '{}' not found.\n\nAvailable ports: {}",
                port_name, available_ports
            );
        }
        
        // Port in use error
        if error_str.contains("in use") || error_str.contains("busy") {
            return format!(
                "Serial port '{}' is already in use by another application.\n\n\
                Close any other programs that might be using this port and try again.",
                port_name
            );
        }
        
        // Generic MS/TP communication error
        format!("MS/TP communication error on '{}': {}", port_name, error_str)
    }

}
