use log::*;
use pnet::datalink::{self, NetworkInterface};

use std::{
    sync::{mpsc, Arc},
    thread,
};

use crate::{
    packet::{PacketReaderFactory, PacketSenderFactory},
    scanners::{arp_scanner, syn_scanner},
};

use super::{syn_scanner::SYNTarget, SYNScanResult, ScanMessage, Scanner};

// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner {
    interface: Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Vec<String>,
    ports: Vec<String>,
    vendor: bool,
    host: bool,
    sender: mpsc::Sender<ScanMessage>,
}

// Returns a new instance of ARPScanner
pub fn new(
    interface: Arc<NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Vec<String>,
    ports: Vec<String>,
    vendor: bool,
    host: bool,
    sender: mpsc::Sender<ScanMessage>,
) -> FullScanner {
    FullScanner {
        interface,
        packet_reader_factory,
        packet_sender_factory,
        targets,
        ports,
        vendor,
        host,
        sender,
    }
}

impl FullScanner {
    fn get_syn_targets_from_arp_scan(&self) -> Vec<SYNTarget> {
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let mut syn_targets: Vec<SYNTarget> = Vec::new();

        let arp = arp_scanner::new(
            Arc::clone(&self.interface),
            self.packet_reader_factory,
            self.packet_sender_factory,
            self.targets.to_owned(),
            self.vendor,
            self.host,
            tx.clone(),
        );

        arp.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                if let Some(_msg) = msg.is_done() {
                    info!("arp sending complete");
                    break;
                }

                if let Some(arp) = msg.is_arp_message() {
                    info!("received arp message: {:?}", msg);
                    syn_targets.push(SYNTarget {
                        ip: arp.ip.clone(),
                        mac: arp.mac.clone(),
                    });
                }
            }
        }

        syn_targets
    }
}

// Implements the Scanner trait for FullScanner
impl Scanner<SYNScanResult> for FullScanner {
    fn scan(&self) {
        let syn_targets = self.get_syn_targets_from_arp_scan();
        let syn = syn_scanner::new(
            Arc::clone(&self.interface),
            self.packet_reader_factory,
            self.packet_sender_factory,
            syn_targets,
            self.ports.to_owned(),
            self.sender.clone(),
        );

        syn.scan()
    }
}
