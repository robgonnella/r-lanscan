use log::*;
use pnet::{
    datalink,
    packet::{arp, ethernet, Packet},
};

use core::time;
use std::{net, str::FromStr, sync, thread};

use crate::{
    packet::{self, PacketReaderFactory, PacketSenderFactory},
    scanners::{Device, IDLE_TIMEOUT},
    targets::{self, LazyLooper},
};

use super::{DeviceStatus, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct ARPScanner {
    interface: sync::Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: sync::Arc<targets::ips::IPTargets>,
    include_vendor: bool,
    include_host_names: bool,
    sender: sync::mpsc::Sender<ScanMessage>,
}

// Returns a new instance of ARPScanner
pub fn new(
    interface: sync::Arc<datalink::NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: sync::Arc<targets::ips::IPTargets>,
    vendor: bool,
    host: bool,
    sender: sync::mpsc::Sender<ScanMessage>,
) -> ARPScanner {
    ARPScanner {
        interface,
        packet_reader_factory,
        packet_sender_factory,
        targets,
        include_vendor: vendor,
        include_host_names: host,
        sender,
    }
}

impl ARPScanner {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done: sync::mpsc::Receiver<()>) {
        let interface = sync::Arc::clone(&self.interface);
        let mut packet_reader = (self.packet_reader_factory)(sync::Arc::clone(&self.interface));
        let include_host_names = self.include_host_names.clone();
        let include_vendor = self.include_vendor.clone();
        let sender = self.sender.clone();

        thread::spawn(move || {
            while let Ok(pkt) = packet_reader.next_packet() {
                if let Ok(_) = done.try_recv() {
                    debug!("exiting arp packet reader");
                    break;
                }

                let eth = &ethernet::EthernetPacket::new(pkt);

                if let Some(eth) = eth {
                    let header = arp::ArpPacket::new(eth.payload());

                    if let Some(header) = header {
                        let op = header.get_operation();
                        let this_mac = interface.mac.unwrap();

                        if op == arp::ArpOperations::Reply && eth.get_source() != this_mac {
                            let sender = sender.clone();
                            let ip4 = header.get_sender_proto_addr();
                            let mac = eth.get_source().to_string();
                            thread::spawn(move || {
                                let mut hostname = String::from("");
                                if include_host_names {
                                    debug!("looking up hostname for {}", ip4.to_string());
                                    if let Ok(host) = dns_lookup::lookup_addr(&ip4.into()) {
                                        hostname = host;
                                    }
                                }
                                let mut vendor = String::from("");
                                if include_vendor {
                                    if let Some(vendor_data) = oui_data::lookup(&mac) {
                                        vendor = vendor_data.organization().to_owned();
                                    }
                                }
                                sender
                                    .send(ScanMessage::ARPScanResult(Device {
                                        hostname,
                                        ip: ip4.to_string(),
                                        mac,
                                        status: DeviceStatus::Online,
                                        vendor,
                                    }))
                                    .unwrap();
                            });
                        }
                    }
                }
            }
        });
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner<Device> for ARPScanner {
    fn scan(&self) {
        debug!("performing ARP scan on targets: {:?}", self.targets);
        debug!("include_vendor: {}", self.include_vendor);
        debug!("include_host_names: {}", self.include_host_names);
        debug!("starting arp packet reader");
        let (done_tx, done_rx) = sync::mpsc::channel::<()>();
        let mut packet_sender = (self.packet_sender_factory)(sync::Arc::clone(&self.interface));
        let msg_sender = self.sender.clone();
        let interface = sync::Arc::clone(&self.interface);
        let targets = sync::Arc::clone(&self.targets);

        self.read_packets(done_rx);

        thread::spawn(move || {
            let process_target = |t: String| {
                thread::sleep(time::Duration::from_micros(100));
                debug!("scanning ARP target: {}", t);
                let target_ipv4 = net::Ipv4Addr::from_str(&t).unwrap();
                let source_ipv4 = interface
                    .ips
                    .iter()
                    .find_map(|ip| match ip.ip() {
                        net::IpAddr::V4(addr) => Some(addr),
                        net::IpAddr::V6(_) => None,
                    })
                    .unwrap();

                let pkt_buf = packet::arp::new(source_ipv4, interface.mac.unwrap(), target_ipv4);
                // Send to the broadcast address
                packet_sender.send(&pkt_buf).unwrap();
            };

            targets.lazy_loop(process_target);

            // TODO make idleTimeout configurable
            thread::sleep(IDLE_TIMEOUT);
            // run your function here
            done_tx.send(()).unwrap();
            msg_sender.send(ScanMessage::Done(())).unwrap();
        });
    }
}
