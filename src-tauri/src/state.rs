// Shared application state for Tauri

use baccy_app::{DeviceManager, ObjectManager, PropertyManager, TransportConfig, TrendingManager};
use baccy_protocol::BacnetService;
use baccy_transport::{BacnetIpTransport, BacnetMstpTransport};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Application settings for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Last used transport configuration
    pub last_transport: Option<TransportConfig>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_transport: None,
        }
    }
}

/// Shared application state
pub struct AppState {
    pub device_manager: Arc<Mutex<Option<DeviceManager>>>,
    pub object_manager: Arc<Mutex<Option<ObjectManager>>>,
    pub property_manager: Arc<Mutex<Option<PropertyManager>>>,
    pub trending_manager: Arc<Mutex<Option<TrendingManager>>>,
    pub service: Arc<Mutex<Option<Arc<BacnetService>>>>,
    pub settings: Arc<Mutex<AppSettings>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            device_manager: Arc::new(Mutex::new(None)),
            object_manager: Arc::new(Mutex::new(None)),
            property_manager: Arc::new(Mutex::new(None)),
            trending_manager: Arc::new(Mutex::new(None)),
            service: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(AppSettings::default())),
        }
    }

    /// Initialize the BACnet service with the selected network interface
    pub fn initialize_service(&self, ip: std::net::Ipv4Addr, port: u16, timeout_ms: u64) -> Result<(), String> {
        let bind_addr = std::net::SocketAddr::new(std::net::IpAddr::V4(ip), port);
        
        let transport = BacnetIpTransport::bind(bind_addr)
            .map_err(|e| format!("Failed to bind to {}: {}", bind_addr, e))?;
        
        let timeout = Duration::from_millis(timeout_ms);
        let service = Arc::new(BacnetService::new(Arc::new(transport), timeout));
        
        // Initialize managers
        let device_manager = DeviceManager::new(Arc::clone(&service));
        let object_manager = ObjectManager::new(Arc::clone(&service));
        let property_manager = PropertyManager::new(Arc::clone(&service));
        let trending_manager = TrendingManager::new(Arc::clone(&service));
        
        // Store in state
        *self.service.lock().unwrap() = Some(service);
        *self.device_manager.lock().unwrap() = Some(device_manager);
        *self.object_manager.lock().unwrap() = Some(object_manager);
        *self.property_manager.lock().unwrap() = Some(property_manager);
        *self.trending_manager.lock().unwrap() = Some(trending_manager);
        
        // Save transport configuration
        let config = TransportConfig::new_ip(bind_addr);
        self.settings.lock().unwrap().last_transport = Some(config);
        
        tracing::info!("BACnet service initialized on {}", bind_addr);
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
        let transport = BacnetMstpTransport::new(&port_name, baud_rate, local_mac)
            .map_err(|e| Self::format_mstp_error(&e, &port_name))?;
        
        let timeout = Duration::from_millis(timeout_ms);
        let service = Arc::new(BacnetService::new(Arc::new(transport), timeout));
        
        // Initialize managers
        let device_manager = DeviceManager::new(Arc::clone(&service));
        let object_manager = ObjectManager::new(Arc::clone(&service));
        let property_manager = PropertyManager::new(Arc::clone(&service));
        let trending_manager = TrendingManager::new(Arc::clone(&service));
        
        // Store in state
        *self.service.lock().unwrap() = Some(service);
        *self.device_manager.lock().unwrap() = Some(device_manager);
        *self.object_manager.lock().unwrap() = Some(object_manager);
        *self.property_manager.lock().unwrap() = Some(property_manager);
        *self.trending_manager.lock().unwrap() = Some(trending_manager);
        
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

    /// Get the last used transport configuration
    pub fn get_last_transport(&self) -> Option<TransportConfig> {
        self.settings.lock().unwrap().last_transport.clone()
    }
}
