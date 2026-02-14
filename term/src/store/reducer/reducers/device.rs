//! Device state reducers for managing discovered network devices.

use r_lanlib::scanners::{Device, Port};
use std::collections::HashSet;

use crate::store::state::State;

/// Adds or updates a single device, merging open ports if already known.
pub fn add_device(state: &mut State, device: Device) {
    if let Some(found_device) = state.device_map.get_mut(&device.ip) {
        let ports: HashSet<Port> = device
            .open_ports
            .0
            .into_iter()
            .chain(found_device.open_ports.0.iter().cloned())
            .collect();
        found_device.open_ports = ports.into();
    } else {
        state.device_map.insert(device.ip, device);
    }
}
