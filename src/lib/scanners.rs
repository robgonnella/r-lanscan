use std::sync::mpsc;

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
pub enum ScanMessageType {
    ARPResult,
    ARPDone,
    SYNResult,
    SYNDone,
}

pub enum ScanMessagePayload {
    ARPScanResult(ARPScanResult),
    SYNScanResult(SYNScanResult),
}

pub struct ScanMessage {
    pub message_type: ScanMessageType,
    pub payload: ScanMessagePayload,
}

pub trait Scanner<T> {
    fn scan(&self) -> mpsc::Receiver<ScanMessage>;
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
