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
    pub fn is_arp_message(&self) -> bool {
        mem::discriminant(&ScanMessage::ARPScanResult(ARPScanResult {
            hostname: String::from(""),
            ip: String::from(""),
            mac: String::from(""),
            status: DeviceStatus::Online,
            vendor: String::from(""),
        })) == mem::discriminant(self)
    }

    pub fn is_syn_message(&self) -> bool {
        mem::discriminant(&ScanMessage::SYNScanResult(SYNScanResult {
            device: ARPScanResult {
                hostname: String::from(""),
                ip: String::from(""),
                mac: String::from(""),
                status: DeviceStatus::Online,
                vendor: String::from(""),
            },
            port: String::from(""),
            port_service: String::from(""),
            port_status: PortStatus::Closed,
        })) == mem::discriminant(self)
    }

    pub fn is_done(&self) -> bool {
        mem::discriminant(&ScanMessage::Done(())) == mem::discriminant(self)
    }
}

pub trait Scanner<T> {
    fn scan(&self);
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
