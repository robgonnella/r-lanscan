use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use std::str::FromStr;

use super::*;

fn default_args(debug: bool) -> Args {
    Args {
        debug,
        ports: vec!["80".to_string()],
    }
}

fn mock_interface() -> NetworkInterface {
    NetworkInterface {
        cidr: "192.168.1.1/24".to_string(),
        description: "test interface".to_string(),
        flags: 0,
        index: 0,
        ips: vec![],
        ipv4: Ipv4Addr::from_str("192.168.1.2").unwrap(),
        mac: MacAddr::default(),
        name: "test_interface".to_string(),
    }
}

#[test]
fn test_initialize_logger() {
    let args = default_args(false);
    initialize_logger(&args).unwrap();
}

#[test]
fn test_get_project_config_path() {
    let p = get_project_config_path().unwrap();
    assert_ne!(p, "");
}

#[test]
fn test_init() {
    let args = default_args(false);
    let interface = mock_interface();
    let (_config, _store) = init(&args, &interface).unwrap();
}
