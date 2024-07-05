use std::{net::Ipv4Addr, str::FromStr};

use ipnet::{Ipv4Net, Ipv4Subnets};

use super::LazyLooper;

#[derive(Debug)]
pub struct IPTargets<'a>(&'a Vec<String>);

pub fn new<'a>(list: &'a Vec<String>) -> IPTargets {
    IPTargets(list)
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
