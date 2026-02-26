//! Network monitoring and scanning orchestration.
//!
//! Runs continuous ARP and SYN scans to discover devices and open ports on
//! the local network.

use color_eyre::eyre::{Result, eyre};
use derive_builder::Builder;
use r_lanlib::{
    network::{self, NetworkInterface},
    packet::wire::Wire,
    scanners::{
        Device, IDLE_TIMEOUT, PortSet, ScanMessage, Scanner,
        arp_scanner::ARPScanner, syn_scanner::SYNScanner,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use std::{
    cell::RefCell,
    collections::HashMap,
    net::Ipv4Addr,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    thread,
    time::{self, Duration},
};

use crate::{
    config::Config,
    error,
    ipc::{
        message::{MainMessage, NetworkMessage},
        network::NetworkIpc,
    },
    process::network::traits::NetworkMonitor,
};

/// Tracks how many scans a device has been missing from.
pub type MissedCount = i8;

const MAX_ARP_MISS: i8 = 3;

/// Data type for monitoring network for devices and open ports.
/// Relays info back to the main thread via ipc.
#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct NetworkProcess {
    wire: Wire,
    interface: Arc<NetworkInterface>,
    ipc: NetworkIpc,
    config: RefCell<Config>,
    throttle: Duration,
    /// Default gateway IP, resolved once at construction time
    #[builder(default)]
    gateway: Option<Ipv4Addr>,
    #[builder(default)]
    arp_history: RefCell<HashMap<Ipv4Addr, (Device, MissedCount)>>,
}

impl NetworkProcess {
    pub fn builder() -> NetworkProcessBuilder {
        NetworkProcessBuilder::default()
    }

    /// Returns devices detected in the last ARP scan (miss count = 0).
    fn get_latest_detected_arp_devices(&self) -> Vec<Device> {
        self.arp_history
            .borrow()
            .iter()
            .filter(|d| d.1.1 == 0)
            .map(|d| d.1.0.clone())
            .collect()
    }

    /// Includes devices that were detected before but may have been missed
    /// for some reason in more recent scans.
    fn get_padded_list_of_arp_devices(&self) -> Vec<Device> {
        self.arp_history
            .borrow()
            .iter()
            .filter(|d| d.1.1 <= MAX_ARP_MISS)
            .map(|d| d.1.0.clone())
            .collect()
    }

    /// Runs an ARP scan and dispatches discovered devices to the store.
    fn process_arp(
        &self,
        scanner: ARPScanner,
        rx: Receiver<ScanMessage>,
    ) -> Result<Receiver<ScanMessage>> {
        self.ipc.tx.send(MainMessage::ArpStart)?;

        let mut arp_results = HashMap::new();

        let handle = scanner.scan()?;

        loop {
            let msg = rx.recv()?;

            match msg {
                ScanMessage::Done => {
                    break;
                }
                ScanMessage::ARPScanDevice(d) => {
                    arp_results.insert(d.ip, d.clone());
                    self.arp_history.borrow_mut().insert(d.ip, (d.clone(), 0));
                    self.ipc.tx.send(MainMessage::ArpUpdate(d))?;
                }
                _ => {}
            }
        }

        handle.join().map_err(error::report_from_thread_panic)??;

        self.ipc.tx.send(MainMessage::ArpDone)?;

        self.arp_history.borrow_mut().iter_mut().for_each(|d| {
            if !arp_results.contains_key(d.0) {
                d.1.1 += 1;
            }
        });

        self.arp_history
            .borrow_mut()
            .retain(|_ip, t| t.1 <= MAX_ARP_MISS);

        Ok(rx)
    }

    /// Runs a SYN scan on arp devices and returns devices with open ports.
    fn process_syn(
        &self,
        scanner: SYNScanner,
        rx: Receiver<ScanMessage>,
    ) -> Result<()> {
        self.ipc.tx.send(MainMessage::SynStart)?;

        // force include arp devices that were detected before but were
        // missed in previous scans up to max allowed misses.
        let arp_devices = self.get_padded_list_of_arp_devices();
        let mut syn_results: HashMap<Ipv4Addr, Device> = HashMap::new();

        for d in arp_devices.iter() {
            syn_results.insert(
                d.ip,
                Device {
                    hostname: d.hostname.to_owned(),
                    ip: d.ip.to_owned(),
                    mac: d.mac.to_owned(),
                    vendor: d.vendor.to_owned(),
                    is_current_host: d.is_current_host,
                    is_gateway: d.is_gateway,
                    open_ports: PortSet::new(),
                    latency_ms: d.latency_ms,
                },
            );
        }

        let handle = scanner.scan()?;

        loop {
            let msg = rx.recv()?;

            match msg {
                ScanMessage::Done => {
                    break;
                }
                ScanMessage::SYNScanDevice(device) => {
                    self.ipc.tx.send(MainMessage::SynUpdate(device.clone()))?;
                }
                _ => {}
            }
        }

        handle.join().map_err(error::report_from_thread_panic)??;

        self.ipc.tx.send(MainMessage::SynDone)
    }
}

impl NetworkMonitor for NetworkProcess {
    /// Main network monitoring loop. Continuously runs ARP and SYN scans,
    /// notifying main thread with discovered devices.
    fn monitor(&self) -> Result<()> {
        loop {
            if let Ok(msg) = self.ipc.rx.try_recv() {
                match msg {
                    NetworkMessage::Quit => return Ok(()),
                    NetworkMessage::ConfigUpdate(config) => {
                        self.config.replace(config);
                    }
                }
            }

            let ip_targets =
                IPTargets::new(vec![Arc::clone(&self.interface).cidr.clone()])
                    .map_err(|e| eyre!("Invalid IP targets: {}", e))?;

            let source_port = network::get_available_port()?;

            let (tx, rx) = mpsc::channel::<ScanMessage>();

            let arp_scanner = ARPScanner::builder()
                .interface(Arc::clone(&self.interface))
                .wire(self.wire.clone())
                .targets(ip_targets)
                .include_host_names(true)
                .include_vendor(true)
                .idle_timeout(time::Duration::from_millis(IDLE_TIMEOUT.into()))
                .source_port(source_port)
                .notifier(tx.clone())
                .gateway(self.gateway)
                .throttle(self.throttle)
                .build()?;

            let rx = self.process_arp(arp_scanner, rx)?;

            let arp_devices = self.get_latest_detected_arp_devices();

            let port_targets =
                PortTargets::new(self.config.borrow().ports.clone())
                    .map_err(|e| eyre!("Invalid port targets: {}", e))?;

            let syn_scanner = SYNScanner::builder()
                .interface(Arc::clone(&self.interface))
                .wire(self.wire.clone())
                .targets(arp_devices)
                .ports(port_targets)
                .source_port(source_port)
                .idle_timeout(time::Duration::from_millis(IDLE_TIMEOUT.into()))
                .notifier(tx.clone())
                .throttle(self.throttle)
                .build()?;

            self.process_syn(syn_scanner, rx)?;

            thread::sleep(time::Duration::from_secs(15));
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
