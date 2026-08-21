//! Provides helpers for managing IP target lists

use std::{net, str::FromStr, sync::Arc};

use crate::error::{RLanLibError, Result};

#[derive(Debug)]
/// Represents a list of IP targets
///
/// This wrapper is used to cut down on the memory needed to store entire
/// network IP ranges. Rather than storing all 65536 IPs in a /16 CIDR block, or
/// a range of IPS, this wrapper allows the storage of just CIDR or range in
/// string form and then dynamically loops the IPs in that block when needed.
///
/// # Errors
///
/// Returns an error if an item in the list is not a valid IP, CIDR block, or range
///
/// # Examples
///
/// ```
/// # use std::net;
/// # use r_lanlib::error::Result;
/// # use r_lanlib::targets::ips::IPTargets;
/// let print_ip = |ip: net::Ipv4Addr| -> Result<()> {
///   println!("ip: {}", ip);
///   Ok(())
/// };
/// let ips = IPTargets::new(
///     vec![
///       "192.168.0.1".to_string(),
///       "172.17.0.1-172.17.0.24".to_string(),
///       "192.168.68.1/24".to_string(),
///     ]
/// ).unwrap();
/// ips.lazy_loop(print_ip).unwrap();
/// ```
pub struct IPTargets(Vec<String>, usize);

fn loop_ips<F: FnMut(net::Ipv4Addr) -> Result<()>>(
    list: &[String],
    mut cb: F,
) -> Result<()> {
    for target in list.iter() {
        if target.contains("-") {
            // target is range
            let parts: Vec<&str> = target.split("-").collect();

            let begin = net::Ipv4Addr::from_str(parts[0]).map_err(|e| {
                RLanLibError::from_net_addr_parse_error(target, e)
            })?;

            let end = net::Ipv4Addr::from_str(parts[1]).map_err(|e| {
                RLanLibError::from_net_addr_parse_error(target, e)
            })?;

            let subnet = ipnet::Ipv4Subnets::new(begin, end, 32);

            for ip_net in subnet {
                for ip in ip_net.hosts() {
                    cb(ip)?;
                }
            }
        } else if target.contains("/") {
            // target is cidr block
            let ip_net = ipnet::Ipv4Net::from_str(target).map_err(|e| {
                RLanLibError::from_ipnet_addr_parse_error(target, e)
            })?;

            for ip in ip_net.hosts() {
                cb(ip)?;
            }
        } else {
            // target is ip
            let ip: net::Ipv4Addr =
                net::Ipv4Addr::from_str(target).map_err(|e| {
                    RLanLibError::from_net_addr_parse_error(target, e)
                })?;

            cb(ip)?;
        }
    }
    Ok(())
}

impl IPTargets {
    /// Returns a new instance of IPTargets using the provided list
    pub fn new(list: Vec<String>) -> Result<Arc<Self>> {
        let mut len = 0;

        loop_ips(&list, |_| {
            len += 1;
            Ok(())
        })?;

        Ok(Arc::new(Self(list, len)))
    }

    /// Returns the true length of the target list. If the underlying
    /// `Vec<String>` is just `["192.168.0.1/24"]`, then a call to "len" will
    /// return 256
    pub fn len(&self) -> usize {
        self.1
    }

    /// Returns true if the list is empty
    pub fn is_empty(&self) -> bool {
        self.1 == 0
    }

    /// loops over all targets including those that are not explicitly in the
    /// list but fall within a range or CIDR block defined in the list
    pub fn lazy_loop<F: FnMut(net::Ipv4Addr) -> Result<()>>(
        &self,
        cb: F,
    ) -> Result<()> {
        loop_ips(&self.0, cb)
    }
}

#[cfg(test)]
#[path = "./ips_tests.rs"]
mod tests;
