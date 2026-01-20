use itertools::Itertools;
use r_lanlib::scanners::{Device, DeviceWithPorts};
use std::{collections::HashMap, net::Ipv4Addr};

use crate::{config::DeviceConfig, ui::store::state::State};

const MAX_ARP_MISS: i8 = 3;

pub fn update_all_devices(state: &mut State, devices: Vec<DeviceWithPorts>) {
    let mut new_map: HashMap<Ipv4Addr, DeviceWithPorts> = HashMap::new();
    let mut arp_history: HashMap<Ipv4Addr, (Device, i8)> = HashMap::new();

    for d in devices.iter() {
        new_map.insert(d.ip, d.clone());
    }

    state.devices = devices.clone();
    state.devices.sort_by_key(|i| i.ip);

    state.device_map = new_map;

    // keep devices that may have been missed in last scan but
    // up to a max limit of misses
    for d in state.arp_history.iter() {
        let mut count = d.1.1;
        if !state.device_map.contains_key(d.0) {
            count += 1;
        }

        if count < MAX_ARP_MISS {
            arp_history.insert(d.0.to_owned(), (d.1.0.clone(), count));
        }
    }

    state.arp_history = arp_history;
}

pub fn add_device(state: &mut State, device: DeviceWithPorts) {
    let arp_device: Device = device.clone().into();

    state.arp_history.insert(device.ip, (arp_device, 0));

    if let std::collections::hash_map::Entry::Vacant(e) = state.device_map.entry(device.ip) {
        state.devices.push(device.clone());
        e.insert(device.clone());
    } else if let Some(found_device) = state.devices.iter_mut().find(|d| d.ip == device.ip) {
        found_device.hostname = device.hostname.clone();
        found_device.ip = device.ip;
        found_device.mac = device.mac;

        for p in &device.open_ports {
            found_device.open_ports.insert(p.clone());
        }

        found_device.open_ports.iter().sorted_by_key(|p| p.id);
        if let Some(mapped_device) = state.device_map.get_mut(&device.ip) {
            *mapped_device = found_device.clone();
        }
    }

    state.devices.sort_by_key(|i| i.ip);
}

pub fn update_selected_device(state: &mut State, ip: Ipv4Addr) {
    if let Some(device) = state.device_map.get(&ip) {
        state.selected_device = Some(device.clone());

        let device_config = if let Some(device_config) =
            state.config.device_configs.get(&device.ip.to_string())
        {
            device_config.clone()
        } else if let Some(device_config) = state.config.device_configs.get(&device.mac.to_string())
        {
            device_config.clone()
        } else {
            DeviceConfig {
                id: device.mac.to_string(),
                ssh_identity_file: state.config.default_ssh_identity.clone(),
                ssh_port: state.config.default_ssh_port,
                ssh_user: state.config.default_ssh_user.clone(),
            }
        };

        state.selected_device_config = Some(device_config);
    }
}
