//! Provides data structure an implementations for performing network scanning
//!
//! This includes:
//! - ARP Scanning
//! - SYN Scanning
//! - Full Scanning (ARP + SYN)

use itertools::Itertools;
#[cfg(test)]
use mockall::{automock, predicate::*};

use pnet::util::MacAddr;
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;
use std::net::Ipv4Addr;
use std::thread::JoinHandle;

use crate::error::Result;

/// The default idle timeout for a scanner
pub const IDLE_TIMEOUT: u16 = 10000;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
/// Data structure representing a port
pub struct Port {
    /// The ID of the port i.e. 22, 80, 443 etc.
    pub id: u16,
    /// The associated service name for the port if known
    pub service: String,
}

impl Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.service.is_empty() {
            write!(f, "{}", self.id)
        } else {
            write!(f, "{}:{}", self.id, self.service)
        }
    }
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Port {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Ord for Port {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Port {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Wrapper around HashSet<Port> providing a convenience method for
/// converting to a Vec of sorted ports
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortSet(pub HashSet<Port>);

impl PortSet {
    /// Returns a new instance of PortSet
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// Returns a sorted Vec of [`Port`]
    pub fn to_sorted_vec(&self) -> Vec<Port> {
        self.0.iter().cloned().sorted().collect()
    }
}

impl From<HashSet<Port>> for PortSet {
    fn from(value: HashSet<Port>) -> Self {
        Self(value)
    }
}

fn serialize_to_string<S, T>(val: &T, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: std::fmt::Display,
{
    s.serialize_str(&val.to_string())
}

fn deserialize_from_str<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s = String::deserialize(d)?;
    s.parse::<T>().map_err(serde::de::Error::custom)
}

// ARP Result from a single device
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
/// Data structure representing a device on the network
pub struct Device {
    /// Hostname of the device
    pub hostname: String,
    /// IPv4 of the device
    pub ip: Ipv4Addr,
    /// MAC address of the device
    #[serde(
        serialize_with = "serialize_to_string",
        deserialize_with = "deserialize_from_str"
    )]
    pub mac: MacAddr,
    /// Vendor of the device if known
    pub vendor: String,
    /// Whether or not the device is the current host running the scan
    pub is_current_host: bool,
    /// A HashSet of open ports for this device
    pub open_ports: PortSet,
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.mac == other.mac
    }
}

impl Hash for Device {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ip.hash(state);
        self.mac.hash(state);
    }
}

impl Ord for Device {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ip.cmp(&other.ip)
    }
}

impl PartialOrd for Device {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
/// Data structure representing a message that a device is being scanned
pub struct Scanning {
    /// IPv4 of the device
    pub ip: Ipv4Addr,
    /// Port being scanned
    pub port: Option<u16>,
}

#[derive(Debug)]
/// Generic enum representing the various kinds of scanning messages over the
/// mcsp channel
pub enum ScanMessage {
    /// Indicates that scanning has completed
    Done,
    /// Send to inform that a device is about to be scanned
    Info(Scanning),
    /// Sent whenever an ARP response is received from a device
    ARPScanDevice(Device),
    /// Sent whenever a SYN response is received from a device
    SYNScanDevice(Device),
}

#[cfg_attr(test, automock)]
/// Trait used by all scanners
pub trait Scanner: Sync + Send {
    /// Performs network scanning
    fn scan(&self) -> JoinHandle<Result<()>>;
}

pub mod arp_scanner;
pub mod full_scanner;
mod heartbeat;
pub mod syn_scanner;
