use crate::scanners::ScanError;

use super::LazyLooper;
use std::{net, str::FromStr, sync::Arc};

#[derive(Debug)]
pub struct IPTargets(Vec<String>, usize);

fn loop_ips<F: FnMut(String) -> Result<(), ScanError>>(
    list: &Vec<String>,
    mut cb: F,
) -> Result<(), ScanError> {
    for target in list.iter() {
        if target.contains("-") {
            // target is range
            let parts: Vec<&str> = target.split("-").collect();

            let begin = net::Ipv4Addr::from_str(parts[0]).or_else(|e| {
                Err(ScanError {
                    ip: target.to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;

            let end = net::Ipv4Addr::from_str(parts[1]).or_else(|e| {
                Err(ScanError {
                    ip: target.to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;

            let subnet = ipnet::Ipv4Subnets::new(begin, end, 32);

            for (_, ip_net) in subnet.enumerate() {
                for ip in ip_net.hosts() {
                    if let Err(err) = cb(ip.to_string()) {
                        return Err(err);
                    }
                }
            }
        } else if target.contains("/") {
            // target is cidr block
            let ip_net = ipnet::Ipv4Net::from_str(&target).or_else(|e| {
                Err(ScanError {
                    ip: target.to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;

            for ip in ip_net.hosts() {
                if let Err(err) = cb(ip.to_string()) {
                    return Err(err);
                }
            }
        } else {
            // target is ip
            let ip: net::Ipv4Addr = net::Ipv4Addr::from_str(&target).or_else(|e| {
                Err(ScanError {
                    ip: target.to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;
            if let Err(err) = cb(ip.to_string()) {
                return Err(err);
            }
        }
    }
    Ok(())
}

impl IPTargets {
    pub fn new(list: Vec<String>) -> Arc<Self> {
        let mut len = 0;

        loop_ips(&list, |_| {
            len += 1;
            Ok(())
        })
        .unwrap();

        Arc::new(Self(list, len))
    }
}

impl LazyLooper<String> for IPTargets {
    fn len(&self) -> usize {
        self.1
    }

    fn lazy_loop<F: FnMut(String) -> Result<(), ScanError>>(&self, cb: F) -> Result<(), ScanError> {
        loop_ips(&self.0, cb)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn returns_new_ip_targets() {
        let list = vec![
            String::from("192.128.28.1"),
            String::from("192.128.28.2"),
            String::from("192.128.28.3"),
        ];
        let targets = IPTargets::new(list);
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

        let targets = IPTargets::new(list);

        let mut idx = 0;

        let assert_ips = |ip: String| {
            assert_eq!(ip, expected[idx]);
            idx += 1;
            Ok(())
        };

        targets.lazy_loop(assert_ips).unwrap();
    }
}
