// Shared application state for Tauri

use baccy_app::{DeviceManager, ObjectManager, PropertyManager, TrendingManager};
use baccy_protocol::BacnetService;
use baccy_transport::BacnetIpTransport;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Shared application state
pub struct AppState {
    pub device_manager: Arc<Mutex<Option<DeviceManager>>>,
    pub object_manager: Arc<Mutex<Option<ObjectManager>>>,
    pub property_manager: Arc<Mutex<Option<PropertyManager>>>,
    pub trending_manager: Arc<Mutex<Option<TrendingManager>>>,
    pub service: Arc<Mutex<Option<Arc<BacnetService>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            device_manager: Arc::new(Mutex::new(None)),
            object_manager: Arc::new(Mutex::new(None)),
            property_manager: Arc::new(Mutex::new(None)),
            trending_manager: Arc::new(Mutex::new(None)),
            service: Arc::new(Mutex::new(None)),
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
        
        tracing::info!("BACnet service initialized on {}", bind_addr);
        Ok(())
    }
}
