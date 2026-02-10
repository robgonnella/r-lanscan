//! Device state reducers for managing discovered network devices.

use itertools::Itertools;
use r_lanlib::scanners::{Device, Port};
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
};

use crate::store::state::State;

/// Replaces all devices after a full scan, tracking devices that went missing.
pub fn update_all_devices(
    state: &mut State,
    devices: HashMap<Ipv4Addr, Device>,
) {
    state.sorted_device_list = devices.values().cloned().sorted().collect();
    state.device_map = devices;
}

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

    state.sorted_device_list =
        state.device_map.values().cloned().sorted().collect();
}
