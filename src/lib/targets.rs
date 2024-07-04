use std::{net::Ipv4Addr, str::FromStr};

use ipnet::{Ipv4Net, Ipv4Subnets};

pub trait LazyLooper<T> {
    fn lazy_loop(self, cb: impl Fn(T));
}

#[derive(Debug)]
pub struct IPTargets<'a>(&'a Vec<String>);

impl<'a> IPTargets<'a> {
    pub fn new(list: &'a Vec<String>) -> IPTargets {
        IPTargets(list)
    }
}

impl<'a> LazyLooper<String> for IPTargets<'a> {
    fn lazy_loop(self, cb: impl Fn(String)) {
        for target in self.0 {
            if target.contains("-") {
                // target is range
                let parts: Vec<&str> = target.split("-").collect();
                let begin = Ipv4Addr::from_str(parts[0]).unwrap();
                let end = Ipv4Addr::from_str(parts[1]).unwrap();
                let subnet = Ipv4Subnets::new(begin, end, 32);
                for (_, ip_net) in subnet.enumerate() {
                    for ip in ip_net.hosts() {
                        cb(ip.to_string())
                    }
                }
            } else if target.contains("/") {
                // target is cidr block
                let ip_net = Ipv4Net::from_str(target).unwrap();
                for ip in ip_net.hosts() {
                    cb(ip.to_string());
                }
            } else {
                // target is ip
                let ip: Ipv4Addr = Ipv4Addr::from_str(target).unwrap();
                cb(ip.to_string());
            }
        }
    }
}

#[derive(Debug)]
pub struct PortTargets<'a>(&'a Vec<String>);

impl<'a> PortTargets<'a> {
    pub fn new(list: &'a Vec<String>) -> PortTargets {
        PortTargets(list)
    }
}

impl<'a> LazyLooper<u32> for PortTargets<'a> {
    fn lazy_loop(self, cb: impl Fn(u32)) {
        for target in self.0 {
            if target.contains("-") {
                let parts: Vec<&str> = target.split("-").collect();
                let begin = parts[0].parse::<u32>().unwrap();
                let end = parts[1].parse::<u32>().unwrap();
                for port in begin..end {
                    cb(port)
                }
            } else {
                let port = target.parse::<u32>().unwrap();
                cb(port)
            }
        }
    }
}
