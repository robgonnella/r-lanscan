use log::*;

use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    packet,
    scanners::{ARPScanResult, DeviceStatus},
    targets::{self, LazyLooper},
};

use super::{ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct ARPScanner {
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<String>,
    include_vendor: bool,
    include_host_names: bool,
    sender: mpsc::Sender<ScanMessage>,
}

// Returns a new instance of ARPScanner
pub fn new(
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<String>,
    vendor: bool,
    host: bool,
    sender: mpsc::Sender<ScanMessage>,
) -> ARPScanner {
    ARPScanner {
        reader,
        targets,
        include_vendor: vendor,
        include_host_names: host,
        sender,
    }
}

impl ARPScanner {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self) {
        let reader_clone = Arc::clone(&self.reader);
        let sender_clone = self.sender.clone();

        thread::spawn(move || {
            let mut reader = reader_clone.lock().unwrap();

            while let Ok(_packet) = reader.next_packet() {
                info!("updating arp packet timeout");

                info!("sending arp result");
                sender_clone
                    .send(ScanMessage::ARPScanResult(ARPScanResult {
                        hostname: String::from("hostname"),
                        ip: String::from("ip"),
                        mac: String::from("mac"),
                        vendor: String::from("vendor"),
                        status: DeviceStatus::Online,
                    }))
                    .unwrap();
            }
        });
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner<ARPScanResult> for ARPScanner {
    fn scan(&self) {
        info!("performing ARP scan on targets: {:?}", self.targets);
        info!("include_vendor: {}", self.include_vendor);
        info!("include_host_names: {}", self.include_host_names);
        info!("starting arp packet reader");

        self.read_packets();

        let target_list = targets::ips::new(&self.targets);

        let process_target = |t: String| {
            info!("processing ARP target: {}", t);
        };

        target_list.lazy_loop(process_target);

        // TODO make idleTimeout configurable
        thread::sleep(Duration::from_secs(5));
        self.sender.send(ScanMessage::Done(())).unwrap();
    }
}
