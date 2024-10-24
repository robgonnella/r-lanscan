use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::thread::JoinHandle;

use serde;
use serde::{Deserialize, Serialize};

pub const IDLE_TIMEOUT: u16 = 10000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortStatus {
    Closed,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Port {
    pub id: u16,
    pub service: String,
    pub status: PortStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceStatus {
    Offline,
    Online,
}

// ARP Result from a single device
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Device {
    pub hostname: String,
    pub ip: String,
    pub mac: String,
    pub status: DeviceStatus,
    pub vendor: String,
}

// Device with open ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWithPorts {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
    pub open_ports: HashSet<Port>,
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

pub trait Scanner: Sync + Send {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
