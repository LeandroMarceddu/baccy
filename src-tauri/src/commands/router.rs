use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub network: u16,
    pub next_hop: String,
    pub interface: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub network: u16,
}

/// Get all router routes
#[tauri::command]
pub fn get_router_routes(state: State<'_, AppState>) -> Result<Vec<RouteInfo>, String> {
    let router = state.router.lock().unwrap();
    let routes = router.routes();
    Ok(routes
        .into_iter()
        .map(|r| RouteInfo {
            network: r.network,
            next_hop: r.next_hop.map(|a| a.to_string()).unwrap_or_default(),
            interface: r.interface,
        })
        .collect())
}

/// Get router interfaces
#[tauri::command]
pub fn get_router_interfaces(state: State<'_, AppState>) -> Result<Vec<InterfaceInfo>, String> {
    let router = state.router.lock().unwrap();
    Ok(router
        .interfaces()
        .iter()
        .map(|(name, _, net)| InterfaceInfo {
            name: name.clone(),
            network: *net,
        })
        .collect())
}

/// Add a route to the router table
#[tauri::command]
pub fn add_router_route(
    network: u16,
    next_hop: String,
    interface: usize,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let next_hop_addr = if next_hop.is_empty() {
        None
    } else {
        Some(
            baccy_core::Address::Ip(
                next_hop
                    .parse::<std::net::SocketAddr>()
                    .map_err(|e| format!("Invalid address: {}", e))?,
            ),
        )
    };
    let router = state.router.lock().unwrap();
    router.add_route(network, next_hop_addr, interface);
    tracing::info!(network, next_hop, interface, "Router route added");
    Ok(())
}

/// Remove a route from the router table
#[tauri::command]
pub fn remove_router_route(network: u16, state: State<'_, AppState>) -> Result<(), String> {
    let router = state.router.lock().unwrap();
    router.remove_route(network);
    tracing::info!(network, "Router route removed");
    Ok(())
}
