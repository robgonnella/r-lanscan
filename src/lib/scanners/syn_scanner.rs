use log::*;
use pnet::datalink;

use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::{
    packet::{PacketReaderFactory, PacketSenderFactory},
    scanners::{ARPScanResult, DeviceStatus, PortStatus},
    targets::{self, LazyLooper},
};

use super::{SYNScanResult, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct SYNScanner {
    interface: Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
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
    interface: Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
    sender: mpsc::Sender<ScanMessage>,
) -> SYNScanner {
    SYNScanner {
        interface,
        packet_reader_factory,
        packet_sender_factory,
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
    fn read_packets(&self, done_rx: mpsc::Receiver<()>) {
        let mut packet_reader = (self.packet_reader_factory)(Arc::clone(&self.interface));
        let sender = self.sender.clone();

        thread::spawn(move || {
            while let Ok(_packet) = packet_reader.next_packet() {
                if let Ok(_) = done_rx.try_recv() {
                    info!("exiting syn packet reader");
                    break;
                }

                sender
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

        let (done_tx, done_rx) = mpsc::channel::<()>();
        let msg_sender = self.sender.clone();

        self.read_packets(done_rx);

        for target in self.targets.iter() {
            let port_list = targets::ports::new(&self.ports);

            let process_port = |port: u32| {
                info!("scanning SYN target: {}:{}", target.ip, port);
            };

            port_list.lazy_loop(process_port);
        }

        thread::spawn(move || {
            thread::sleep(Duration::from_secs(5));
            // run your function here
            done_tx.send(()).unwrap();
            msg_sender.send(ScanMessage::Done(())).unwrap();
        });
    }
}
