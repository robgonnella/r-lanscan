use log::*;
use std::{
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{PacketReaderFactory, PacketSenderFactory},
    targets::{ips::IPTargets, ports::PortTargets},
};

use super::{
    arp_scanner::ARPScanner, syn_scanner::SYNScanner, Device, ScanError, ScanMessage, Scanner,
};

// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Arc<IPTargets>,
    ports: Arc<PortTargets>,
    vendor: bool,
    host: bool,
    idle_timeout: Duration,
    sender: mpsc::Sender<ScanMessage>,
    source_port: u16,
}

impl<'net> FullScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader_factory: PacketReaderFactory,
        packet_sender_factory: PacketSenderFactory,
        targets: Arc<IPTargets>,
        ports: Arc<PortTargets>,
        vendor: bool,
        host: bool,
        idle_timeout: Duration,
        sender: mpsc::Sender<ScanMessage>,
        source_port: u16,
    ) -> Self {
        Self {
            interface,
            packet_reader_factory,
            packet_sender_factory,
            targets,
            ports,
            vendor,
            host,
            idle_timeout,
            sender,
            source_port,
        }
    }
}

impl<'net> FullScanner<'net> {
    fn get_syn_targets_from_arp_scan(&self) -> Vec<Device> {
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let mut syn_targets: Vec<Device> = Vec::new();

        let arp = ARPScanner::new(
            self.interface,
            self.packet_reader_factory,
            self.packet_sender_factory,
            Arc::clone(&self.targets),
            self.vendor,
            self.host,
            self.idle_timeout,
            tx.clone(),
        );

        arp.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done(_) => {
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
impl<'net> Scanner for FullScanner<'net> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
        let syn_targets = self.get_syn_targets_from_arp_scan();
        let syn = SYNScanner::new(
            self.interface,
            self.packet_reader_factory,
            self.packet_sender_factory,
            syn_targets,
            Arc::clone(&self.ports),
            self.idle_timeout,
            self.sender.clone(),
            self.source_port,
        );

        syn.scan()
    }
}

unsafe impl<'net> Sync for FullScanner<'net> {}
unsafe impl<'net> Send for FullScanner<'net> {}
