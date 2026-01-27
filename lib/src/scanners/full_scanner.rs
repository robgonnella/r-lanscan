//! Provides Scanner implementation for Full scanning (ARP + SYN)

use derive_builder::Builder;
use log::*;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    error::Result,
    network::NetworkInterface,
    packet::{Reader, Sender},
    targets::{ips::IPTargets, ports::PortTargets},
};

use super::{
    Device, ScanMessage, Scanner, arp_scanner::ARPScanner,
    syn_scanner::SYNScanner,
};

/// Data structure representing a Full scanner (ARP + SYN)
#[derive(Builder)]
#[builder(setter(into))]
pub struct FullScanner<'net> {
    /// Network interface to use for scanning
    interface: &'net NetworkInterface,
    /// Packet reader for receiving responses
    packet_reader: Arc<Mutex<dyn Reader>>,
    /// Packet sender for transmitting packets
    packet_sender: Arc<Mutex<dyn Sender>>,
    /// IP targets to scan for device discovery
    targets: Arc<IPTargets>,
    /// Port targets to scan on discovered devices
    ports: Arc<PortTargets>,
    /// Whether to include vendor lookups for discovered devices
    vendor: bool,
    /// Whether to include hostname lookups for discovered devices
    host: bool,
    /// Duration to wait for responses after scanning completes
    idle_timeout: Duration,
    /// Channel for sending scan results and status messages
    notifier: mpsc::Sender<ScanMessage>,
    /// Source port for packet listener and incoming packet identification
    source_port: u16,
}

impl<'n> FullScanner<'n> {
    /// Returns a builder for FullScanner
    pub fn builder() -> FullScannerBuilder<'n> {
        FullScannerBuilder::default()
    }

    fn get_syn_targets_from_arp_scan(&self) -> Result<Vec<Device>> {
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let mut syn_targets: Vec<Device> = Vec::new();

        let arp = ARPScanner::builder()
            .interface(self.interface)
            .packet_reader(Arc::clone(&self.packet_reader))
            .packet_sender(Arc::clone(&self.packet_sender))
            .targets(Arc::clone(&self.targets))
            .source_port(self.source_port)
            .include_vendor(self.vendor)
            .include_host_names(self.host)
            .idle_timeout(self.idle_timeout)
            .notifier(tx.clone())
            .build()?;

        arp.scan()?;

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done => {
                        debug!("arp sending complete");
                        break;
                    }
                    ScanMessage::ARPScanDevice(device) => {
                        syn_targets.push(device.to_owned());
                    }
                    _ => {}
                }
            }
        }

        Ok(syn_targets)
    }
}

// Implements the Scanner trait for FullScanner
impl Scanner for FullScanner<'_> {
    fn scan(&self) -> Result<JoinHandle<Result<()>>> {
        let syn_targets = self.get_syn_targets_from_arp_scan()?;

        let syn = SYNScanner::builder()
            .interface(self.interface)
            .packet_reader(Arc::clone(&self.packet_reader))
            .packet_sender(Arc::clone(&self.packet_sender))
            .targets(syn_targets)
            .ports(Arc::clone(&self.ports))
            .source_port(self.source_port)
            .idle_timeout(self.idle_timeout)
            .notifier(self.notifier.clone())
            .build()?;

        syn.scan()
    }
}

#[cfg(test)]
#[path = "./full_scanner_tests.rs"]
mod tests;
