use std::{
    sync::{mpsc, Arc, Mutex},
    thread::spawn,
};

use crate::{
    packet,
    targets::{self, LazyLooper},
};

use super::{SYNScanResult, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct SYNScanner {
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
}

// SYN Target represents the required fields to send a SYN packet to a device
#[derive(Debug)]
pub struct SYNTarget {
    pub ip: String,
    pub mac: String,
}

// Returns a new instance of SYNScanner
pub fn new(
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
) -> SYNScanner {
    SYNScanner {
        reader,
        targets,
        ports,
    }
}

impl SYNScanner {
    // Allow mutable setting of syn targets
    pub fn set_targets(&mut self, targets: Vec<SYNTarget>) {
        self.targets = targets;
    }

    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self) -> mpsc::Receiver<ScanMessage> {
        let clone = Arc::clone(&self.reader);
        let (_, rx) = mpsc::channel::<ScanMessage>();

        spawn(move || {
            let mut reader = clone.lock().unwrap();
            while let Ok(packet) = reader.next_packet() {
                println!("received ARP packet! {:?}", packet);
            }
        });

        rx
    }
}

// Implements the Scanner trait for SYNScanner
impl Scanner<SYNScanResult> for SYNScanner {
    fn scan(&self) -> mpsc::Receiver<ScanMessage> {
        println!("performing SYN scan on targets: {:?}", self.targets);

        println!("starting syn packet reader");

        let rx = self.read_packets();

        let results: Vec<SYNScanResult> = Vec::new();

        for target in self.targets.iter() {
            let port_list = targets::ports::new(&self.ports);

            let process_port = |port: u32| {
                println!("processing SYN target: {}:{}", target.ip, port);
            };

            port_list.lazy_loop(process_port);
        }

        rx
    }
}
