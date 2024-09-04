use std::{net::TcpListener, sync};

use pnet::datalink::NetworkInterface;

pub fn get_interface(name: &str) -> sync::Arc<NetworkInterface> {
    sync::Arc::new(
        pnet::datalink::interfaces()
            .iter()
            .find(|i| i.name == name)
            .unwrap()
            .to_owned(),
    )
}

pub fn get_default_interface<'a>() -> sync::Arc<NetworkInterface> {
    sync::Arc::new(
        pnet::datalink::interfaces()
            .iter()
            .find(|e| e.is_up() && !e.is_loopback() && e.ips.iter().find(|i| i.is_ipv4()).is_some())
            .unwrap()
            .to_owned(),
    )
}

pub fn get_interface_ipv4(interface: sync::Arc<NetworkInterface>) -> String {
    let ipnet = interface.ips.iter().find(|i| i.is_ipv4()).unwrap();
    ipnet.ip().to_string()
}

pub fn get_interface_cidr(interface: sync::Arc<NetworkInterface>) -> String {
    let ipnet = interface.ips.iter().find(|i| i.is_ipv4()).unwrap();
    let ip = ipnet.ip().to_string();
    let prefix = ipnet.prefix().to_string();
    String::from(format!("{ip}/{prefix}"))
}

pub fn get_available_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    addr.port()
}
