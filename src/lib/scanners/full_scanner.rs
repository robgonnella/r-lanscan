use log::*;
use pnet::datalink::NetworkInterface;

use std::{
    cell::RefCell,
    sync::{mpsc, Arc, Mutex},
};

use crate::{
    packet,
    scanners::{arp_scanner, syn_scanner},
};

use super::{syn_scanner::SYNTarget, SYNScanResult, ScanMessage, Scanner};

// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner {
    arp_receiver: mpsc::Receiver<ScanMessage>,
    arp: arp_scanner::ARPScanner,
    syn: RefCell<syn_scanner::SYNScanner>,
}

// Returns a new instance of ARPScanner
pub fn new(
    interface: Arc<NetworkInterface>,
    packet_reader: Arc<Mutex<Box<dyn packet::Reader>>>,
    packet_sender: Arc<Mutex<Box<dyn packet::Sender>>>,
    targets: Vec<String>,
    ports: Vec<String>,
    vendor: bool,
    host: bool,
    sender: mpsc::Sender<ScanMessage>,
) -> FullScanner {
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    FullScanner {
        arp_receiver: rx,
        // make sure to clone the reader as we want both arp and syn scanners
        // to have access
        arp: arp_scanner::new(
            interface,
            Arc::clone(&packet_reader),
            Arc::clone(&packet_sender),
            targets,
            vendor,
            host,
            tx.clone(),
        ),
        // Here we need the internals of syn_scanner to be mutable in order to
        // call "set_targets" but our outer data structure should still be
        // immutable. To Achieve this we use "RefCell", which is not thread-safe
        // but doesn't need to be for our purpose. If we needed it to be
        // thread-safe we would use Mutex.
        syn: RefCell::new(syn_scanner::new(
            Arc::clone(&packet_reader),
            Arc::clone(&packet_sender),
            vec![],
            ports,
            sender.clone(),
        )),
    }
}

// Implements the Scanner trait for FullScanner
impl Scanner<SYNScanResult> for FullScanner {
    fn scan(&self) {
        let mut syn_targets: Vec<SYNTarget> = Vec::new();

        self.arp.scan();

        loop {
            let msg = self.arp_receiver.recv().unwrap();

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

        self.syn.borrow_mut().set_targets(syn_targets);
        self.syn.borrow().scan()
    }
}
