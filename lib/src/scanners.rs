//! Provides data structure an implementations for performing network scanning
//!
//! This includes:
//! - ARP Scanning
//! - SYN Scanning
//! - Full Scanning (ARP + SYN)

#[cfg(test)]
use mockall::{automock, predicate::*};

use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::thread::JoinHandle;

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

// ARP Result from a single device
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Data structure representing a device on the network
pub struct Device {
    /// Hostname of the device
    pub hostname: String,
    /// IPv4 of the device
    pub ip: String,
    /// MAC address of the device
    pub mac: String,
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
    pub ip: String,
    /// MAC address of the device
    pub mac: String,
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
            ip: value.ip.clone(),
            mac: value.mac.clone(),
            hostname: value.hostname.clone(),
            vendor: value.vendor.clone(),
            is_current_host: value.is_current_host.clone(),
        }
    }
}

#[derive(Debug)]
/// Data structure representing a message that a device is being scanned
pub struct Scanning {
    /// IPv4 of the device
    pub ip: String,
    /// Port being scanned
    pub port: Option<String>,
}

#[derive(Debug)]
/// Data structure representing a message that an error occurred while scanning
pub struct ScanError {
    /// The IPv4 of device being scanned if known
    pub ip: Option<String>,
    /// The port being scanned if known
    pub port: Option<String>,
    /// The error encountered
    pub error: Box<dyn Error>,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ip = self.ip.clone().unwrap_or(String::from(""));
        let port = self.port.clone().unwrap_or(String::from(""));
        let msg = format!(
            "scanning error: ip {ip}, port: {port}, msg: {0}",
            self.error
        );
        write!(f, "{msg}")
    }
}

impl Error for ScanError {}
unsafe impl Send for ScanError {}
unsafe impl Sync for ScanError {}

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
    fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
}

pub mod arp_scanner;
pub mod full_scanner;
mod heartbeat;
pub mod syn_scanner;

#[cfg(test)]
#[path = "./scanners_tests.rs"]
mod tests;
