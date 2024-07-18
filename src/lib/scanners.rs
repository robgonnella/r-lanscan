use core::time;
use std::collections::HashMap;

const IDLE_TIMEOUT: time::Duration = time::Duration::from_secs(5);

// we only discover "online" devices so there is no "offline" status
#[derive(Debug, Clone)]
pub enum DeviceStatus {
    Online,
}

#[derive(Debug, Clone)]
pub enum PortStatus {
    Open,
    Closed,
}

pub type DeviceIp = String;

// ARP Result from a single device
#[derive(Debug, Clone)]
pub struct Device {
    pub ip: DeviceIp,
    pub mac: String,
    pub status: DeviceStatus,
    pub hostname: String,
    pub vendor: String,
}

// SYN Result from a single device
#[derive(Debug)]
pub struct SYNScanResult {
    pub device: Device,
    pub port: String,
    pub port_status: PortStatus,
    pub port_service: String,
}

pub type DeviceHashMap = HashMap<DeviceIp, Device>;

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

pub trait Scanner<T> {
    fn scan(&self);
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
