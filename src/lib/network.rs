use std::{net::Ipv4Addr, str::FromStr};

pub fn get_default_device_name() -> String {
    let device = pcap::Device::lookup()
        .expect("device lookup failed")
        .expect("no device available");
    device.name
}

pub fn get_default_network_cidr() -> String {
    let device = pcap::Device::lookup()
        .expect("device lookup failed")
        .expect("no device available");

    let mut cidr: String = String::from("");

    for a in device.addresses.iter() {
        if a.addr.is_ipv4() && !a.addr.is_loopback() {
            let prefix = netmask_to_bit(&a.netmask.unwrap().to_string());
            let ipv4 = Ipv4Addr::from_str(a.addr.to_string().as_str()).unwrap();
            let net = ipnet::Ipv4Net::new(ipv4, u8::try_from(prefix).ok().unwrap()).unwrap();
            cidr = net.trunc().to_string();
            break;
        }
    }

    cidr
}

// private functions

fn netmask_to_bit(netmask: &str) -> u32 {
    let bits: u32 = netmask
        .split(".")
        .map(|x| x.parse::<u8>().unwrap().count_ones())
        .sum();
    bits
}
