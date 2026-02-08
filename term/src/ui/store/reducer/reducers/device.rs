//! Device state reducers for managing discovered network devices.

use itertools::Itertools;
use r_lanlib::scanners::{Device, Port};
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
};

use crate::ui::store::state::State;

const MAX_ARP_MISS: i8 = 3;

/// Replaces all devices after a full scan, tracking devices that went missing.
pub fn update_all_devices(
    state: &mut State,
    devices: HashMap<Ipv4Addr, Device>,
) {
    let mut new_arp_history: HashMap<Ipv4Addr, (Device, i8)> = HashMap::new();

    state.sorted_device_list = devices.values().cloned().sorted().collect();
    state.device_map = devices;

    // keep devices that may have been missed in last scan but
    // up to a max limit of misses
    for d in state.arp_history.iter() {
        let mut count = d.1.1;
        if !state.device_map.contains_key(d.0) {
            count += 1;
        }

        if count < MAX_ARP_MISS {
            new_arp_history.insert(d.0.to_owned(), (d.1.0.clone(), count));
        }
    }

    state.arp_history = new_arp_history;
}

/// Adds or updates a single device, merging open ports if already known.
pub fn add_device(state: &mut State, device: Device) {
    let arp_device: Device = device.clone();

    state.arp_history.insert(device.ip, (arp_device, 0));

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
