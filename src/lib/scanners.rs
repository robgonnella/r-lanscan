use std::mem;

// we only discover "online" devices so there is no "offline" status
#[derive(Debug)]
pub enum DeviceStatus {
    Online,
}

#[derive(Debug)]
pub enum PortStatus {
    Open,
    Closed,
}

// ARP Result from a single device
#[derive(Debug)]
pub struct ARPScanResult {
    pub ip: String,
    pub mac: String,
    pub status: DeviceStatus,
    pub hostname: String,
    pub vendor: String,
}

// SYN Result from a single device
#[derive(Debug)]
pub struct SYNScanResult {
    pub device: ARPScanResult,
    pub port: String,
    pub port_status: PortStatus,
    pub port_service: String,
}

#[derive(Debug)]
pub enum ScanMessage {
    Done(()),
    ARPScanResult(ARPScanResult),
    SYNScanResult(SYNScanResult),
}

impl ScanMessage {
    pub fn is_arp_message(&self) -> Option<&ARPScanResult> {
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
