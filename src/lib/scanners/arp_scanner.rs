use std::{
    sync::{mpsc, Arc, Mutex},
    thread::spawn,
};

use crate::{
    packet,
    scanners::{ARPScanResult, DeviceStatus, ScanMessagePayload, ScanMessageType},
    targets::{self, LazyLooper},
};

use super::{ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct ARPScanner {
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
    targets: Vec<String>,
    include_vendor: bool,
    include_host_names: bool,
}

// Returns a new instance of ARPScanner
pub fn new(
    reader: Arc<Mutex<Box<dyn packet::Reader + Send + Sync>>>,
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
    fn read_packets(&self) -> mpsc::Receiver<ScanMessage> {
        let clone = Arc::clone(&self.reader);
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        spawn(move || {
            let mut reader = clone.lock().unwrap();
            let mut sent_first_packet = false;
            while let Ok(packet) = reader.next_packet() {
                println!("received ARP packet! {:?}", packet);

                if !sent_first_packet {
                    tx.send(ScanMessage {
                        message_type: ScanMessageType::ARPResult,
                        payload: ScanMessagePayload::ARPScanResult(ARPScanResult {
                            hostname: String::from("hostname"),
                            ip: String::from("ip"),
                            mac: String::from("mac"),
                            vendor: String::from("vendor"),
                            status: DeviceStatus::Online,
                        }),
                    })
                    .unwrap();
                    sent_first_packet = true
                } else {
                    tx.send(ScanMessage {
                        message_type: ScanMessageType::ARPDone,
                        payload: ScanMessagePayload::ARPScanResult(ARPScanResult {
                            hostname: String::from(""),
                            ip: String::from(""),
                            mac: String::from(""),
                            vendor: String::from(""),
                            status: DeviceStatus::Online,
                        }),
                    })
                    .unwrap();
                    break;
                }
            }

            drop(tx);
        });

        rx
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner<ARPScanResult> for ARPScanner {
    fn scan(&self) -> mpsc::Receiver<ScanMessage> {
        println!("performing ARP scan on targets: {:?}", self.targets);
        println!("include_vendor: {}", self.include_vendor);
        println!("include_host_names: {}", self.include_host_names);

        println!("starting arp packet reader");

        let rx = self.read_packets();

        let target_list = targets::ips::new(&self.targets);

        let process_target = |t: String| {
            println!("processing ARP target: {}", t);
        };

        target_list.lazy_loop(process_target);

        rx
    }
}
