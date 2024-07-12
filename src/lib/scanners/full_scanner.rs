use std::{
    cell::RefCell,
    sync::{mpsc, Arc, Mutex},
};

use crate::{
    packet,
    scanners::{arp_scanner, syn_scanner, ScanMessageType},
};

use super::{syn_scanner::SYNTarget, SYNScanResult, ScanMessage, Scanner};

// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner {
    arp: arp_scanner::ARPScanner,
    syn: RefCell<syn_scanner::SYNScanner>,
}

// Returns a new instance of ARPScanner
pub fn new(
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<String>,
    ports: Vec<String>,
    vendor: bool,
    host: bool,
) -> FullScanner {
    FullScanner {
        // make sure to clone the reader as we want both arp and syn scanners
        // to have access
        arp: arp_scanner::new(Arc::clone(&reader), targets, vendor, host),
        // Here we need the internals of syn_scanner to be mutable in order to
        // call "set_targets" but our outer data structure should still be
        // immutable. To Achieve this we use "RefCell", which is not thread-safe
        // but doesn't need to be for our purpose. If we needed it to be
        // thread-safe we would use Mutex.
        syn: RefCell::new(syn_scanner::new(Arc::clone(&reader), vec![], ports)),
    }
}

// Implements the Scanner trait for FullScanner
impl Scanner<SYNScanResult> for FullScanner {
    fn scan(&self) -> mpsc::Receiver<ScanMessage> {
        let mut syn_targets: Vec<SYNTarget> = Vec::new();

        let arp_rx = self.arp.scan();

        let mut arp_done = false;

        while !arp_done {
            let message = arp_rx.recv().unwrap();
            println!("received arp message: {:?}", message);
            if message.message_type == ScanMessageType::ARPDone {
                arp_done = true;
            }
        }

        // let syn_targets = {
        //     let mut v: Vec<SYNTarget> = Vec::new();
        //     for r in results {
        //         v.push(SYNTarget {
        //             ip: r.ip,
        //             mac: r.mac,
        //         });
        //     }
        //     v
        // };

        self.syn.borrow_mut().set_targets(syn_targets);
        self.syn.borrow().scan()
    }
}
