use std::{
    sync::{Arc, Mutex},
    thread::{sleep, spawn},
    time::Duration,
};

use crate::{
    capture,
    targets::{self, LazyLooper},
};

use super::Scanner;

// Data structure representing an ARP scanner
pub struct ARPScanner {
    reader: Arc<Mutex<Box<dyn capture::PacketReader + Send + Sync>>>,
    targets: Vec<String>,
    include_vendor: bool,
    include_host_names: bool,
}

// ARP Result from a single device
#[derive(Debug)]
pub struct ARPScanResult {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
}

// Returns a new instance of ARPScanner
pub fn new(
    reader: Arc<Mutex<Box<dyn capture::PacketReader + Send + Sync>>>,
    targets: Vec<String>,
    vendor: bool,
    host: bool,
) -> ARPScanner {
    ARPScanner {
        reader,
        targets,
        include_vendor: vendor,
        include_host_names: host,
    }
}

impl ARPScanner {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self) {
        let clone = Arc::clone(&self.reader);
        spawn(move || {
            let mut reader = clone.lock().unwrap();
            while let Ok(packet) = reader.next_packet() {
                println!("received ARP packet! {:?}", packet);
            }
        });
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner<ARPScanResult> for ARPScanner {
    fn scan(&self) -> Vec<ARPScanResult> {
        println!("performing ARP scan on targets: {:?}", self.targets);
        println!("include_vendor: {}", self.include_vendor);
        println!("include_host_names: {}", self.include_host_names);

        println!("starting arp packet reader");

        self.read_packets();

        let results: Vec<ARPScanResult> = Vec::new();

        let target_list = targets::ips::new(&self.targets);

        let process_target = |t: String| {
            println!("processing ARP target: {}", t);
        };

        target_list.lazy_loop(process_target);

        sleep(Duration::from_secs(120));

        results
    }
}
