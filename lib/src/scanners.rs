//! Provides data structure an implementations for performing network scanning
//!
//! This includes:
//! - ARP Scanning
//! - SYN Scanning
//! - Full Scanning (ARP + SYN)

#[cfg(test)]
use mockall::{automock, predicate::*};

use pnet::util::MacAddr;
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::thread::JoinHandle;

use crate::error::Result;

/// The default idle timeout for a scanner
pub const IDLE_TIMEOUT: u16 = 10000;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Data structure representing a port
pub struct Port {
    /// The ID of the port i.e. 22, 80, 443 etc.
    pub id: u16,
    /// The associated service name for the port if known
    pub service: String,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

// Device with open ports
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data structure representing a device on the network with detected open ports
pub struct DeviceWithPorts {
    /// IPv4 of the device
    pub ip: Ipv4Addr,
    /// MAC address of the device
    #[serde(
        serialize_with = "serialize_to_string",
        deserialize_with = "deserialize_from_str"
    )]
    pub mac: MacAddr,
    /// Hostname of the device
    pub hostname: String,
    /// Device vendor if known
    pub vendor: String,
    /// Whether or not the device is the current host running the scan
    pub is_current_host: bool,
    /// A list of detected open ports on the device
    pub open_ports: HashSet<Port>,
}

impl From<DeviceWithPorts> for Device {
    fn from(value: DeviceWithPorts) -> Self {
        Self {
            ip: value.ip,
            mac: value.mac,
            hostname: value.hostname.clone(),
            vendor: value.vendor.clone(),
            is_current_host: value.is_current_host,
        }
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

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Data structure representing the result of SYN scan on a device for a port
pub struct SYNScanResult {
    /// The device that was scanned
    pub device: Device,
    /// The port that was scanned
    pub open_port: Port,
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
    ARPScanResult(Device),
    /// Sent whenever a SYN response is received from a device
    SYNScanResult(SYNScanResult),
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

#[cfg(test)]
#[path = "./scanners_tests.rs"]
mod tests;
