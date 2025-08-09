//! Provides helpers for managing IP target lists

use crate::scanners::ScanError;

use std::{net, str::FromStr, sync::Arc};

#[derive(Debug)]
/// Represents a list of IP targets
///
/// This wrapper is used to cut down on the memory needed to store entire
/// network IP ranges. Rather than storing all 65536 IPs in a /16 CIDR block, or
/// a range of IPS, this wrapper allows the storage of just CIDR or range in
/// string form and then dynamically loops the IPs in that block when needed.
///
/// # Panics
///
/// Panics if an item in the list is not a valid IP
///
/// # Examples
///
/// ```rust
/// let ips = IPTargets::new(
///     vec![
///         "192.168.0.1".to_string(),
///         "172.17.0.1-172.17.0.24".to_string(),
///         "192.168.68.1/24".to_string(),
///     ]
/// )?;
/// ```
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
    /// Returns a new instance of IPTargets using the provided list
    pub fn new(list: Vec<String>) -> Arc<Self> {
        let mut len = 0;

        loop_ips(&list, |_| {
            len += 1;
            Ok(())
        })
        .unwrap();

        Arc::new(Self(list, len))
    }

    /// Returns the true length of the target list. If the underlying
    /// `Vec<String>` is just `["192.168.0.1/24"]`, then a call to "len" will
    /// return 256
    pub fn len(&self) -> usize {
        self.1
    }

    /// loops over all targets including those that are not explicitly in the
    /// list but fall within a range or CIDR block defined in the list
    pub fn lazy_loop<F: FnMut(net::Ipv4Addr) -> Result<(), ScanError>>(
        &self,
        cb: F,
    ) -> Result<(), ScanError> {
        loop_ips(&self.0, cb)
    }
}

#[cfg(test)]
#[path = "./ips_tests.rs"]
mod tests;
