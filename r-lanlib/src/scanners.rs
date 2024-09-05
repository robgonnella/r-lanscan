use std::collections::HashSet;

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

// SYN Result from a single device
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SYNScanResult {
    pub device: Device,
    pub open_port: Port,
}

#[derive(Debug)]
pub enum ScanMessage {
    Done(()),
    ARPScanResult(Device),
    SYNScanResult(SYNScanResult),
}

impl ScanMessage {
    pub fn is_arp_message(&self) -> Option<&Device> {
        match self {
            ScanMessage::Done(_msg) => None,
            ScanMessage::SYNScanResult(_msg) => None,
            ScanMessage::ARPScanResult(msg) => Some(msg),
        }
    }

    pub fn is_syn_message(&self) -> Option<&SYNScanResult> {
        match self {
            ScanMessage::Done(_msg) => None,
            ScanMessage::SYNScanResult(msg) => Some(msg),
            ScanMessage::ARPScanResult(_msg) => None,
        }
    }

    pub fn is_done(&self) -> Option<()> {
        match self {
            ScanMessage::Done(_msg) => Some(()),
            ScanMessage::SYNScanResult(_msg) => None,
            ScanMessage::ARPScanResult(_msg) => None,
        }
    }
}

pub trait Scanner: Sync + Send {
    fn scan(&self);
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
