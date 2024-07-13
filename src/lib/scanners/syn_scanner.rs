use log::*;

use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    packet,
    scanners::{ARPScanResult, DeviceStatus, PortStatus},
    targets::{self, LazyLooper},
};

use super::{SYNScanResult, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct SYNScanner {
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
    sender: mpsc::Sender<ScanMessage>,
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
    sender: mpsc::Sender<ScanMessage>,
) -> SYNScanner {
    SYNScanner {
        reader,
        targets,
        ports,
        sender,
    }
}

impl SYNScanner {
    // Allow mutable setting of syn targets
    pub fn set_targets(&mut self, targets: Vec<SYNTarget>) {
        self.targets = targets;
    }

    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self) {
        let clone = Arc::clone(&self.reader);
        let sender_clone = self.sender.clone();

        thread::spawn(move || {
            let mut reader = clone.lock().unwrap();
            while let Ok(_packet) = reader.next_packet() {
                info!("sending syn result");
                sender_clone
                    .send(ScanMessage::SYNScanResult(SYNScanResult {
                        device: ARPScanResult {
                            hostname: String::from("hostname"),
                            ip: String::from("ip"),
                            mac: String::from("mac"),
                            status: DeviceStatus::Online,
                            vendor: String::from("vendor"),
                        },
                        port: String::from("22"),
                        port_service: String::from("ssh"),
                        port_status: PortStatus::Open,
                    }))
                    .unwrap();
            }
        });
    }
}

// Implements the Scanner trait for SYNScanner
impl Scanner<SYNScanResult> for SYNScanner {
    fn scan(&self) {
        info!("performing SYN scan on targets: {:?}", self.targets);

        info!("starting syn packet reader");

        self.read_packets();

        for target in self.targets.iter() {
            let port_list = targets::ports::new(&self.ports);

            let process_port = |port: u32| {
                info!("processing SYN target: {}:{}", target.ip, port);
            };

            port_list.lazy_loop(process_port);
        }

        // TODO make idleTimeout configurable
        thread::sleep(Duration::from_secs(5));
        self.sender.send(ScanMessage::Done(())).unwrap();
    }
}
