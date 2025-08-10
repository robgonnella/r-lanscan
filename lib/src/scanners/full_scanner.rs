//! Provides Scanner implementation for Full scanning (ARP + SYN)

use log::*;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{Reader, Sender},
    targets::{ips::IPTargets, ports::PortTargets},
};

use super::{
    arp_scanner::{ARPScanner, ARPScannerArgs},
    syn_scanner::{SYNScanner, SYNScannerArgs},
    Device, ScanError, ScanMessage, Scanner,
};

/// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Arc<IPTargets>,
    ports: Arc<PortTargets>,
    vendor: bool,
    host: bool,
    idle_timeout: Duration,
    notifier: mpsc::Sender<ScanMessage>,
    source_port: u16,
}

/// Data structure holding parameters needed to create instance of FullScanner
pub struct FullScannerArgs<'net> {
    /// The network interface to use when scanning
    pub interface: &'net NetworkInterface,
    /// A packet Reader implementation (can use default provided in packet
    /// crate)
    pub packet_reader: Arc<Mutex<dyn Reader>>,
    /// A packet Sender implementation (can use default provided in packet
    /// crate)
    pub packet_sender: Arc<Mutex<dyn Sender>>,
    /// [`IPTargets`] to scan
    pub targets: Arc<IPTargets>,
    /// [`PortTargets`] to scan for each detected device
    pub ports: Arc<PortTargets>,
    /// An open source port to listen for incoming packets (can use network
    /// packet to find open port)
    pub source_port: u16,
    /// Whether or not to include vendor look-ups for detected devices
    pub include_vendor: bool,
    /// Whether or not to include hostname look-ups for detected devices
    pub include_host_names: bool,
    /// The amount of time to wait for incoming packets after scanning all
    /// targets
    pub idle_timeout: Duration,
    /// Channel to send messages regarding devices being scanned, and detected
    /// devices
    pub notifier: mpsc::Sender<ScanMessage>,
}

impl<'net> FullScanner<'net> {
    /// Returns a new instance of FullScanner using provided info
    pub fn new(args: FullScannerArgs<'net>) -> Self {
        Self {
            interface: args.interface,
            packet_reader: args.packet_reader,
            packet_sender: args.packet_sender,
            targets: args.targets,
            ports: args.ports,
            vendor: args.include_vendor,
            host: args.include_host_names,
            idle_timeout: args.idle_timeout,
            notifier: args.notifier,
            source_port: args.source_port,
        }
    }
}

impl FullScanner<'_> {
    fn get_syn_targets_from_arp_scan(&self) -> Vec<Device> {
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let mut syn_targets: Vec<Device> = Vec::new();

        let arp = ARPScanner::new(ARPScannerArgs {
            interface: self.interface,
            packet_reader: Arc::clone(&self.packet_reader),
            packet_sender: Arc::clone(&self.packet_sender),
            targets: Arc::clone(&self.targets),
            source_port: self.source_port,
            include_vendor: self.vendor,
            include_host_names: self.host,
            idle_timeout: self.idle_timeout,
            notifier: tx.clone(),
        });

        arp.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done => {
                        debug!("arp sending complete");
                        break;
                    }
                    ScanMessage::ARPScanResult(device) => {
                        syn_targets.push(device.to_owned());
                    }
                    _ => {}
                }
            }
        }

        syn_targets
    }
}

// Implements the Scanner trait for FullScanner
impl Scanner for FullScanner<'_> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
        let syn_targets = self.get_syn_targets_from_arp_scan();
        let syn = SYNScanner::new(SYNScannerArgs {
            interface: self.interface,
            packet_reader: Arc::clone(&self.packet_reader),
            packet_sender: Arc::clone(&self.packet_sender),
            targets: syn_targets,
            ports: Arc::clone(&self.ports),
            source_port: self.source_port,
            idle_timeout: self.idle_timeout,
            notifier: self.notifier.clone(),
        });

        syn.scan()
    }
}

#[cfg(test)]
#[path = "./full_scanner_tests.rs"]
mod tests;
