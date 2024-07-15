use std::{net::Ipv4Addr, sync::Arc};

use pnet::{datalink::NetworkInterface, util::MacAddr};

#[derive(Debug)]
pub struct Interface {
    pub name: String,
    pub mac: MacAddr,
    pub ipv4: Ipv4Addr,
    pub cidr: String,
}

pub fn get_interface(name: &str) -> Arc<NetworkInterface> {
    Arc::new(
        pnet::datalink::interfaces()
            .iter()
            .find(|i| i.name == name)
            .unwrap()
            .to_owned(),
    )
}

pub fn get_default_interface<'a>() -> Arc<NetworkInterface> {
    Arc::new(
        pnet::datalink::interfaces()
            .iter()
            .find(|e| e.is_up() && !e.is_loopback() && e.ips.iter().find(|i| i.is_ipv4()).is_some())
            .unwrap()
            .to_owned(),
    )
}

pub fn get_interface_cidr(interface: Arc<NetworkInterface>) -> String {
    let ipnet = interface.ips.iter().find(|i| i.is_ipv4()).unwrap();
    let ip = ipnet.ip().to_string();
    let prefix = ipnet.prefix().to_string();
    String::from(format!("{ip}/{prefix}"))
}
