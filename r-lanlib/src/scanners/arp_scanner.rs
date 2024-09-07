use log::*;
use pnet::{
    packet::{arp, ethernet, Packet},
    util::MacAddr,
};

use core::time;
use std::{
    net,
    str::FromStr,
    sync::{self, Arc},
    thread,
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{arp::ARPPacket, PacketReaderFactory, PacketSenderFactory},
    scanners::Device,
    targets::{ips::IPTargets, LazyLooper},
};

use super::{DeviceStatus, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct ARPScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Arc<IPTargets>,
    include_vendor: bool,
    include_host_names: bool,
    idle_timeout: Duration,
    sender: sync::mpsc::Sender<ScanMessage>,
}

impl<'net> ARPScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader_factory: PacketReaderFactory,
        packet_sender_factory: PacketSenderFactory,
        targets: Arc<IPTargets>,
        vendor: bool,
        host: bool,
        idle_timeout: Duration,
        sender: sync::mpsc::Sender<ScanMessage>,
    ) -> Self {
        Self {
            interface,
            packet_reader_factory,
            packet_sender_factory,
            targets,
            include_vendor: vendor,
            include_host_names: host,
            idle_timeout,
            sender,
        }
    }
}

impl<'net> ARPScanner<'net> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done: sync::mpsc::Receiver<()>, source_mac: MacAddr) {
        let mut packet_reader = (self.packet_reader_factory)(&self.interface);
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

                        if op == arp::ArpOperations::Reply && eth.get_source() != source_mac {
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
                                    .expect("failed to send ScanMessage::ARPScanResult");
                            });
                        }
                    }
                }
            }
        });
    }
}

// Implements the Scanner trait for ARPScanner
impl<'net> Scanner for ARPScanner<'net> {
    fn scan(&self) {
        debug!("performing ARP scan on targets: {:?}", self.targets);
        debug!("include_vendor: {}", self.include_vendor);
        debug!("include_host_names: {}", self.include_host_names);
        debug!("starting arp packet reader");
        let (done_tx, done_rx) = sync::mpsc::channel::<()>();
        let mut packet_sender = (self.packet_sender_factory)(&self.interface);
        let msg_sender = self.sender.clone();
        let idle_timeout = self.idle_timeout;
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let targets = Arc::clone(&self.targets);

        self.read_packets(done_rx, source_mac.clone());

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || {
            let process_target = |t: String| {
                thread::sleep(time::Duration::from_micros(100));
                debug!("scanning ARP target: {}", t);
                let target_ipv4 = net::Ipv4Addr::from_str(&t).expect("failed to parse ip target");
                let pkt_buf = ARPPacket::new(source_ipv4, source_mac, target_ipv4);
                // Send to the broadcast address
                packet_sender.send(&pkt_buf).unwrap();
            };

            targets.lazy_loop(process_target);

            thread::sleep(idle_timeout);
            done_tx.send(()).unwrap();
            msg_sender.send(ScanMessage::Done(())).unwrap();
        });
    }
}

unsafe impl<'net> Sync for ARPScanner<'net> {}
unsafe impl<'net> Send for ARPScanner<'net> {}
