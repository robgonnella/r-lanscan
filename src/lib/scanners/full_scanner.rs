use log::*;
use pnet::datalink;

use std::{collections::HashMap, sync};

use crate::{
    packet::{PacketReaderFactory, PacketSenderFactory},
    scanners::{arp_scanner, syn_scanner},
    targets,
};

use super::{Device, DeviceHashMap, SYNScanResult, ScanMessage, Scanner};

// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner {
    interface: sync::Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: sync::Arc<targets::ips::IPTargets>,
    ports: sync::Arc<targets::ports::PortTargets>,
    vendor: bool,
    host: bool,
    sender: sync::mpsc::Sender<ScanMessage>,
}

// Returns a new instance of ARPScanner
pub fn new<'targets, 'ports>(
    interface: sync::Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: sync::Arc<targets::ips::IPTargets>,
    ports: sync::Arc<targets::ports::PortTargets>,
    vendor: bool,
    host: bool,
    sender: sync::mpsc::Sender<ScanMessage>,
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
    fn get_syn_targets_from_arp_scan(&self) -> DeviceHashMap {
        let (tx, rx) = sync::mpsc::channel::<ScanMessage>();

        let mut syn_targets: HashMap<String, Device> = HashMap::new();

        let arp = arp_scanner::new(
            sync::Arc::clone(&self.interface),
            self.packet_reader_factory,
            self.packet_sender_factory,
            sync::Arc::clone(&self.targets),
            self.vendor,
            self.host,
            tx.clone(),
        );

        arp.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                if let Some(_msg) = msg.is_done() {
                    debug!("arp sending complete");
                    break;
                }

                if let Some(device) = msg.is_arp_message() {
                    debug!("received arp message: {:?}", msg);
                    syn_targets.insert(device.ip.to_owned(), device.to_owned());
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
            sync::Arc::clone(&self.interface),
            self.packet_reader_factory,
            self.packet_sender_factory,
            sync::Arc::new(syn_targets),
            sync::Arc::clone(&self.ports),
            self.sender.clone(),
        );

        syn.scan()
    }
}
