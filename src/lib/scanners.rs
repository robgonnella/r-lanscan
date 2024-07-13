use std::sync::mpsc;

// we only discover "online" devices so there is no "offline" status
#[derive(Debug, PartialEq)]
pub enum DeviceStatus {
    Online,
}

#[derive(Debug, PartialEq)]
pub enum PortStatus {
    Open,
    Closed,
}

// ARP Result from a single device
#[derive(Debug, PartialEq)]
pub struct ARPScanResult {
    pub ip: String,
    pub mac: String,
    pub status: DeviceStatus,
    pub hostname: String,
    pub vendor: String,
}

// SYN Result from a single device
#[derive(Debug, PartialEq)]
pub struct SYNScanResult {
    pub device: ARPScanResult,
    pub port: String,
    pub port_status: PortStatus,
    pub port_service: String,
}

#[derive(Debug, PartialEq)]
pub enum ScanMessage {
    Done(()),
    ARPScanResult(ARPScanResult),
    SYNScanResult(SYNScanResult),
}

pub trait Scanner<T> {
    fn scan(&self) -> mpsc::Receiver<ScanMessage>;
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
