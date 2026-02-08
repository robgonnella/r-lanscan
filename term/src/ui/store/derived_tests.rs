use nanoid::nanoid;
use pnet::util::MacAddr;
use r_lanlib::scanners::{Device, PortSet};
use std::{
    collections::HashMap,
    fs,
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};

use crate::{
    config::{Config, ConfigManager},
    ui::store::{Dispatcher, StateGetter, Store, action::Action},
};

use super::*;

fn tear_down(conf_path: &str) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_get_detected_devices() {
    let device_1 = Device {
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        hostname: "fancy_hostname".to_string(),
        vendor: "mac".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_2 = Device {
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        hostname: "super_fancy_hostname".to_string(),
        vendor: "linux".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_3 = Device {
        ip: Ipv4Addr::new(10, 10, 10, 3),
        mac: MacAddr::new(0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa),
        hostname: "just_ok_hostname".to_string(),
        vendor: "linux".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());

    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();
    let config_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();
    let conf_manager = Arc::new(Mutex::new(config_manager));
    let config = Config::new(user, identity, cidr);
    let store = Store::new(conf_manager, config.clone());

    store
        .dispatch(Action::CreateAndSetConfig(config.clone()))
        .unwrap();

    // this updates arp history
    store.dispatch(Action::AddDevice(device_1.clone())).unwrap();
    store.dispatch(Action::AddDevice(device_2.clone())).unwrap();
    store.dispatch(Action::AddDevice(device_3.clone())).unwrap();

    let mut devices = HashMap::new();
    devices.insert(device_1.ip, device_1.clone());
    devices.insert(device_2.ip, device_2.clone());
    devices.insert(device_3.ip, device_3.clone());

    store
        .dispatch(Action::UpdateAllDevices(devices.clone()))
        .unwrap();

    // missed dev2 & dev3
    devices.remove(&device_2.ip);
    devices.remove(&device_3.ip);
    store
        .dispatch(Action::UpdateAllDevices(devices.clone()))
        .unwrap();

    // missed dev3
    devices.insert(device_2.ip, device_2.clone());
    store
        .dispatch(Action::UpdateAllDevices(devices.clone()))
        .unwrap();

    // missed dev3 again
    store
        .dispatch(Action::UpdateAllDevices(devices.clone()))
        .unwrap();

    // missed dev3 again
    store.dispatch(Action::UpdateAllDevices(devices)).unwrap();

    // get just devices with miss count = 0 -> device_1
    let arp_devices = get_detected_arp_devices(&store.get_state().unwrap());
    assert_eq!(arp_devices.len(), 1);
    assert_eq!(arp_devices[0], device_1);
    tear_down(tmp_path.as_str());
}
