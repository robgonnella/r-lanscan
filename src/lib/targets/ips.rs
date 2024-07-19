use std::{net, str::FromStr, sync};

use super::LazyLooper;

#[derive(Debug)]
pub struct IPTargets(Vec<String>);

pub fn new(list: Vec<String>) -> sync::Arc<IPTargets> {
    sync::Arc::new(IPTargets(list))
}

impl LazyLooper<String> for IPTargets {
    fn lazy_loop<F: FnMut(String)>(&self, mut cb: F) {
        for target in self.0.iter() {
            if target.contains("-") {
                // target is range
                let parts: Vec<&str> = target.split("-").collect();
                let begin = net::Ipv4Addr::from_str(parts[0]).unwrap();
                let end = net::Ipv4Addr::from_str(parts[1]).unwrap();
                let subnet = ipnet::Ipv4Subnets::new(begin, end, 32);
                for (_, ip_net) in subnet.enumerate() {
                    for ip in ip_net.hosts() {
                        cb(ip.to_string())
                    }
                }
            } else if target.contains("/") {
                // target is cidr block
                let ip_net = ipnet::Ipv4Net::from_str(&target).unwrap();
                for ip in ip_net.hosts() {
                    cb(ip.to_string());
                }
            } else {
                // target is ip
                let ip: net::Ipv4Addr = net::Ipv4Addr::from_str(&target).unwrap();
                cb(ip.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn returns_new_ip_targets() {
        let list = vec![String::from("1"), String::from("2"), String::from("3")];
        let targets = new(list);
        assert!(!targets.0.is_empty());
    }

    #[test]
    fn lazy_loops_ips() {
        let list = vec![
            String::from("192.128.28.1"),
            String::from("192.128.28.2-192.128.28.4"),
            String::from("192.128.30.0/30"),
        ];

        let expected = [
            String::from("192.128.28.1"),
            String::from("192.128.28.2"),
            String::from("192.128.28.3"),
            String::from("192.128.28.4"),
            String::from("192.128.30.1"),
            String::from("192.128.30.2"),
        ];

        let targets = new(list);

        let mut idx = 0;

        let assert_ips = |ip: String| {
            assert_eq!(ip, expected[idx]);
            idx += 1;
        };

        targets.lazy_loop(assert_ips);
    }
}
