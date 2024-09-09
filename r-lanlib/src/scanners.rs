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
    pub ip: String,
    pub port: Option<String>,
    pub msg: String,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut msg = format!("failed to scan target: {0}", self.ip);

        if self.port.is_some() {
            let port = self.port.as_ref().expect("should have port");
            msg = format!("{msg}:{port}");
        }

        msg = format!("{msg}: {0}", self.msg);

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

impl ScanMessage {
    pub fn arp_message(&self) -> Option<&Device> {
        match self {
            ScanMessage::ARPScanResult(msg) => Some(msg),
            _ => None,
        }
    }

    pub fn syn_message(&self) -> Option<&SYNScanResult> {
        match self {
            ScanMessage::SYNScanResult(msg) => Some(msg),
            _ => None,
        }
    }

    pub fn info(&self) -> Option<&Scanning> {
        match self {
            ScanMessage::Info(msg) => Some(msg),
            _ => None,
        }
    }

    pub fn done(&self) -> Option<()> {
        match self {
            ScanMessage::Done(_msg) => Some(()),
            _ => None,
        }
    }
}

pub trait Scanner: Sync + Send {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
