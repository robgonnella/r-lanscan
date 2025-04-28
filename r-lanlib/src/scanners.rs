#[cfg(test)]
use mockall::{automock, predicate::*};

use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::thread::JoinHandle;

pub const IDLE_TIMEOUT: u16 = 10000;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Port {
    pub id: u16,
    pub service: String,
}

// ARP Result from a single device
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Device {
    pub hostname: String,
    pub ip: String,
    pub mac: String,
    pub vendor: String,
    pub is_current_host: bool,
}

// Device with open ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWithPorts {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
    pub is_current_host: bool,
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
pub struct Scanning {
    pub ip: String,
    pub port: Option<String>,
}

#[derive(Debug)]
pub struct ScanError {
    pub ip: Option<String>,
    pub port: Option<String>,
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

// SYN Result from a single device
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SYNScanResult {
    pub device: Device,
    pub open_port: Port,
}

#[derive(Debug)]
pub enum ScanMessage {
    Done(()),
    Info(Scanning),
    ARPScanResult(Device),
    SYNScanResult(SYNScanResult),
}

#[cfg_attr(test, automock)]
pub trait Scanner: Sync + Send {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
}

pub mod arp_scanner;
pub mod full_scanner;
mod heartbeat;
pub mod syn_scanner;
