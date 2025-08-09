use crate::scanners::ScanError;

use std::{net, str::FromStr, sync::Arc};

#[derive(Debug)]
pub struct IPTargets(Vec<String>, usize);

fn loop_ips<F: FnMut(net::Ipv4Addr) -> Result<(), ScanError>>(
    list: &Vec<String>,
    mut cb: F,
) -> Result<(), ScanError> {
    for target in list.iter() {
        if target.contains("-") {
            // target is range
            let parts: Vec<&str> = target.split("-").collect();

            let begin = net::Ipv4Addr::from_str(parts[0]).or_else(|e| {
                Err(ScanError {
                    ip: Some(target.to_string()),
                    port: None,
                    error: Box::from(e),
                })
            })?;

            let end = net::Ipv4Addr::from_str(parts[1]).or_else(|e| {
                Err(ScanError {
                    ip: Some(target.to_string()),
                    port: None,
                    error: Box::from(e),
                })
            })?;

            let subnet = ipnet::Ipv4Subnets::new(begin, end, 32);

            for (_, ip_net) in subnet.enumerate() {
                for ip in ip_net.hosts() {
                    cb(ip)?;
                }
            }
        } else if target.contains("/") {
            // target is cidr block
            let ip_net = ipnet::Ipv4Net::from_str(&target).or_else(|e| {
                Err(ScanError {
                    ip: Some(target.to_string()),
                    port: None,
                    error: Box::from(e),
                })
            })?;

            for ip in ip_net.hosts() {
                cb(ip)?;
            }
        } else {
            // target is ip
            let ip: net::Ipv4Addr = net::Ipv4Addr::from_str(&target).or_else(|e| {
                Err(ScanError {
                    ip: Some(target.to_string()),
                    port: None,
                    error: Box::from(e),
                })
            })?;
            cb(ip)?;
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

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn lazy_loop<F: FnMut(net::Ipv4Addr) -> Result<(), ScanError>>(
        &self,
        cb: F,
    ) -> Result<(), ScanError> {
        loop_ips(&self.0, cb)
    }
}

#[cfg(test)]
#[path = "./tests/ips_tests.rs"]
mod tests;
