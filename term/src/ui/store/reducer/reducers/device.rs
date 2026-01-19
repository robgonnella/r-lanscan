use std::{collections::HashMap, net::Ipv4Addr, str::FromStr};

use itertools::Itertools;
use r_lanlib::scanners::{Device, DeviceWithPorts};

use crate::{config::DeviceConfig, ui::store::state::State};

const MAX_ARP_MISS: i8 = 3;

pub fn update_all_devices(state: &mut State, devices: Vec<DeviceWithPorts>) {
    let mut new_map: HashMap<String, DeviceWithPorts> = HashMap::new();
    let mut arp_history: HashMap<String, (Device, i8)> = HashMap::new();

    for d in devices.iter() {
        new_map.insert(d.ip.clone(), d.clone());
    }

    state.devices = devices.clone();
    state
        .devices
        .sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());

    state.device_map = new_map;

    // keep devices that may have been missed in last scan but
    // up to a max limit of misses
    for d in state.arp_history.iter() {
        let mut count = d.1.1;
        if !state.device_map.contains_key(d.0) {
            count += 1;
        }

        if count < MAX_ARP_MISS {
            arp_history.insert(d.0.clone(), (d.1.0.clone(), count));
        }
    }

    state.arp_history = arp_history;
}

pub fn add_device(state: &mut State, device: DeviceWithPorts) {
    let arp_device: Device = device.clone().into();

    state.arp_history.insert(device.ip.clone(), (arp_device, 0));

    if let std::collections::hash_map::Entry::Vacant(e) = state.device_map.entry(device.ip.clone())
    {
        state.devices.push(device.clone());
        e.insert(device.clone());
    } else {
        let found_device = state
            .devices
            .iter_mut()
            .find(|d| d.ip == device.ip)
            .unwrap();
        found_device.hostname = device.hostname.clone();
        found_device.ip = device.ip.clone();
        found_device.mac = device.mac.clone();

        for p in &device.open_ports {
            found_device.open_ports.insert(p.clone());
        }

        found_device.open_ports.iter().sorted_by_key(|p| p.id);
        let mapped_device = state.device_map.get_mut(&device.ip.clone()).unwrap();
        *mapped_device = found_device.clone();
    }

    state
        .devices
        .sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());
}

pub fn update_selected_device(state: &mut State, ip: String) {
    if let Some(device) = state.device_map.get(ip.as_str()) {
        state.selected_device = Some(device.clone());
        let device_config: DeviceConfig;
        if state.config.device_configs.contains_key(&device.ip) {
            device_config = state.config.device_configs.get(&device.ip).unwrap().clone();
        } else if state.config.device_configs.contains_key(&device.mac) {
            device_config = state
                .config
                .device_configs
                .get(&device.mac)
                .unwrap()
                .clone();
        } else {
            device_config = DeviceConfig {
                id: device.mac.clone(),
                ssh_identity_file: state.config.default_ssh_identity.clone(),
                ssh_port: state
                    .config
                    .default_ssh_port
                    .clone()
                    .parse::<u16>()
                    .unwrap(),
                ssh_user: state.config.default_ssh_user.clone(),
            }
        }

        state.selected_device_config = Some(device_config);
    }
}
