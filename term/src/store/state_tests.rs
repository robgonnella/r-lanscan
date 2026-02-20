use pnet::util::MacAddr;
use r_lanlib::scanners::{Device, PortSet};
use std::net::Ipv4Addr;

use super::State;

#[test]
fn test_device_list_returns_sorted_by_ip() {
    let mut state = State::default();

    // Add devices in non-sorted order
    let dev1 = Device {
        hostname: "dev1".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 3),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "vendor1".to_string(),
        latency_ms: None,
    };

    let dev2 = Device {
        hostname: "dev2".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "vendor2".to_string(),
        latency_ms: None,
    };

    let dev3 = Device {
        hostname: "dev3".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "vendor3".to_string(),
        latency_ms: None,
    };

    // Insert in non-sorted order
    state.device_map.insert(dev1.ip, dev1.clone());
    state.device_map.insert(dev2.ip, dev2.clone());
    state.device_map.insert(dev3.ip, dev3.clone());

    let list = state.device_list();

    // Verify sorted order by IP
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].ip, Ipv4Addr::new(10, 10, 10, 1));
    assert_eq!(list[1].ip, Ipv4Addr::new(10, 10, 10, 2));
    assert_eq!(list[2].ip, Ipv4Addr::new(10, 10, 10, 3));
}

#[test]
fn test_device_list_empty() {
    let state = State::default();
    let list = state.device_list();
    assert_eq!(list.len(), 0);
    assert!(list.is_empty());
}

#[test]
fn test_device_list_single_device() {
    let mut state = State::default();

    let dev = Device {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "vendor".to_string(),
        latency_ms: None,
    };

    state.device_map.insert(dev.ip, dev.clone());

    let list = state.device_list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].ip, dev.ip);
}
