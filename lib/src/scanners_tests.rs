use std::{collections::HashSet, net::Ipv4Addr};

use pnet::util::MacAddr;

use crate::scanners::Device;

use super::DeviceWithPorts;

#[test]
fn test_device_from_device_with_ports() {
    let dev_with_ports = DeviceWithPorts {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 10),
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        is_current_host: false,
        open_ports: HashSet::new(),
    };

    let dev: Device = dev_with_ports.clone().into();

    assert_eq!(dev.hostname, dev_with_ports.hostname);
    assert_eq!(dev.ip, dev_with_ports.ip);
    assert_eq!(dev.mac, dev_with_ports.mac);
    assert_eq!(dev.vendor, dev_with_ports.vendor);
    assert_eq!(dev.is_current_host, dev_with_ports.is_current_host);
}
