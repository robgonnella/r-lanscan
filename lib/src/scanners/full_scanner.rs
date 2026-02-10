//! Provides Scanner implementation for Full scanning (ARP + SYN)

use derive_builder::Builder;
use std::{
    sync::{Arc, mpsc},
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    error::Result,
    network::NetworkInterface,
    packet::wire::Wire,
    targets::{ips::IPTargets, ports::PortTargets},
};

use super::{
    Device, ScanMessage, Scanner, arp_scanner::ARPScanner,
    syn_scanner::SYNScanner,
};

/// Data structure representing a Full scanner (ARP + SYN)
#[derive(Builder)]
#[builder(setter(into))]
pub struct FullScanner {
    /// Network interface to use for scanning
    interface: Arc<NetworkInterface>,
    /// Wire for reading and sending packets on the wire
    wire: Wire,
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

impl FullScanner {
    /// Returns a builder for FullScanner
    pub fn builder() -> FullScannerBuilder {
        FullScannerBuilder::default()
    }

    fn get_syn_targets_from_arp_scan(&self) -> Result<Vec<Device>> {
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let mut syn_targets: Vec<Device> = Vec::new();

        let arp = ARPScanner::builder()
            .interface(Arc::clone(&self.interface))
            .wire(self.wire.clone())
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
                        log::debug!("arp sending complete");
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
impl Scanner for FullScanner {
    fn scan(&self) -> Result<JoinHandle<Result<()>>> {
        let syn_targets = self.get_syn_targets_from_arp_scan()?;

        let syn = SYNScanner::builder()
            .interface(Arc::clone(&self.interface))
            .wire(self.wire.clone())
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
