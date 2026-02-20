//! Device state reducers for managing discovered network devices.

use r_lanlib::scanners::{Device, Port};
use std::collections::HashSet;

use crate::store::state::{MAX_LATENCY_HISTORY, State};

/// Merges open ports from a SYN scan result into an existing device.
/// Does not update latency_ms or latency_history — those are ARP-only.
pub fn update_device_ports(state: &mut State, device: Device) {
    if let Some(found_device) = state.device_map.get_mut(&device.ip) {
        let ports: HashSet<Port> = device
            .open_ports
            .0
            .into_iter()
            .chain(found_device.open_ports.0.iter().cloned())
            .collect();
        found_device.open_ports = ports.into();
    }
}

/// Adds or updates a single device from an ARP scan, merging open ports,
/// updating latency_ms, and appending to latency_history when available.
pub fn add_device(state: &mut State, device: Device) {
    if let Some(latency) = device.latency_ms {
        let history = state.latency_history.entry(device.ip).or_default();
        if history.len() == MAX_LATENCY_HISTORY {
            history.remove(0);
        }
        history.push(latency as u64);
    }

    if let Some(found_device) = state.device_map.get_mut(&device.ip) {
        let ports: HashSet<Port> = device
            .open_ports
            .0
            .into_iter()
            .chain(found_device.open_ports.0.iter().cloned())
            .collect();
        found_device.open_ports = ports.into();
        if device.latency_ms.is_some() {
            found_device.latency_ms = device.latency_ms;
        }
    } else {
        state.device_map.insert(device.ip, device);
    }
}
