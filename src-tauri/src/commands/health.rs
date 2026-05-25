// Device health/reachability command

use crate::state::AppState;
use baccy_protocol::device_state::DeviceHealth;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn get_device_health(state: State<'_, AppState>) -> Result<HashMap<u32, DeviceHealth>, String> {
    let service = {
        let service_lock = state.service.lock().unwrap();
        service_lock
            .as_ref()
            .ok_or("Service not initialized")?
            .clone()
    };
    let tracker = service.get_device_tracker();
    Ok(tracker
        .get_all_health()
        .into_iter()
        .map(|(id, h)| (id, h))
        .collect())
}
