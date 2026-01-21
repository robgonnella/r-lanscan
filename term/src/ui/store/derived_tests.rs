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
    ui::store::{Dispatcher, Store, action::Action},
};

use super::*;

fn tear_down(conf_path: &str) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_get_device_config_from_state() {
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

    let mut devices = HashMap::new();
    devices.insert(device_1.ip, device_1.clone());
    devices.insert(device_2.ip, device_2.clone());

    store.dispatch(Action::CreateAndSetConfig(config.clone()));
    store.dispatch(Action::UpdateAllDevices(devices));
    store.dispatch(Action::UpdateDeviceConfig(DeviceConfig {
        id: device_1.mac.to_string(),
        ssh_port: 2222,
        ssh_identity_file: "dev_1_id_rsa".to_string(),
        ssh_user: "dev1_user".to_string(),
    }));
    store.dispatch(Action::UpdateSelectedDevice(device_1.ip));

    let state = store.get_state().unwrap();

    let dev1_config = get_selected_device_config_from_state(&state);

    assert_eq!(dev1_config.id, device_1.mac.to_string());
    assert_eq!(dev1_config.ssh_port, 2222);
    assert_eq!(dev1_config.ssh_identity_file, "dev_1_id_rsa");
    assert_eq!(dev1_config.ssh_user, "dev1_user");

    tear_down(tmp_path.as_str());
}

#[test]
fn test_get_device_config_from_state_default() {
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

    let mut devices = HashMap::new();
    devices.insert(device_1.ip, device_1.clone());
    devices.insert(device_2.ip, device_2.clone());

    store.dispatch(Action::CreateAndSetConfig(config.clone()));
    store.dispatch(Action::UpdateAllDevices(devices));
    store.dispatch(Action::UpdateDeviceConfig(DeviceConfig {
        id: device_1.mac.to_string(),
        ssh_port: 2222,
        ssh_identity_file: "dev_1_id_rsa".to_string(),
        ssh_user: "dev1_user".to_string(),
    }));

    let state = store.get_state().unwrap();

    let dev1_config = get_selected_device_config_from_state(&state);

    assert_eq!(dev1_config.id, MacAddr::default().to_string());
    assert_eq!(dev1_config.ssh_port, config.default_ssh_port);
    assert_eq!(dev1_config.ssh_identity_file, config.default_ssh_identity);
    assert_eq!(dev1_config.ssh_user, config.default_ssh_user);

    tear_down(tmp_path.as_str());
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

    store.dispatch(Action::CreateAndSetConfig(config.clone()));

    // this updates arp history
    store.dispatch(Action::AddDevice(device_1.clone()));
    store.dispatch(Action::AddDevice(device_2.clone()));
    store.dispatch(Action::AddDevice(device_3.clone()));

    let mut devices = HashMap::new();
    devices.insert(device_1.ip, device_1.clone());
    devices.insert(device_2.ip, device_2.clone());
    devices.insert(device_3.ip, device_3.clone());

    store.dispatch(Action::UpdateAllDevices(devices.clone()));

    // missed dev2 & dev3
    devices.remove(&device_2.ip);
    devices.remove(&device_3.ip);
    store.dispatch(Action::UpdateAllDevices(devices.clone()));

    // missed dev3
    devices.insert(device_2.ip, device_2.clone());
    store.dispatch(Action::UpdateAllDevices(devices.clone()));

    // missed dev3 again
    store.dispatch(Action::UpdateAllDevices(devices.clone()));

    // missed dev3 again
    store.dispatch(Action::UpdateAllDevices(devices));

    // get just devices with miss count = 0 -> device_1
    let arp_devices = get_detected_arp_devices(&store.get_state().unwrap());
    assert_eq!(arp_devices.len(), 1);
    assert_eq!(arp_devices[0], device_1);
    tear_down(tmp_path.as_str());
}
