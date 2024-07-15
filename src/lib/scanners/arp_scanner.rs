use log::*;
use pnet::{
    datalink::NetworkInterface,
    packet::{
        arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket},
        ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket},
        Packet,
    },
    util::MacAddr,
};

use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::{
    packet::{PacketReaderFactory, PacketSenderFactory},
    scanners::ARPScanResult,
    targets::{self, LazyLooper},
};

use super::{DeviceStatus, ScanMessage, Scanner};

// Constants used to help locate our nested packets
const PKT_ETH_SIZE: usize = EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = ArpPacket::minimum_packet_size();
const PKT_ARP_OFFSET: usize = PKT_ETH_SIZE;

// Data structure representing an ARP scanner
pub struct ARPScanner {
    interface: Arc<NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Vec<String>,
    include_vendor: bool,
    include_host_names: bool,
    sender: mpsc::Sender<ScanMessage>,
}

// Returns a new instance of ARPScanner
pub fn new(
    interface: Arc<NetworkInterface>,
    packet_reader_factory: PacketReaderFactory,
    packet_sender_factory: PacketSenderFactory,
    targets: Vec<String>,
    vendor: bool,
    host: bool,
    sender: mpsc::Sender<ScanMessage>,
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
    fn read_packets(&self, done: mpsc::Receiver<()>) {
        let interface = Arc::clone(&self.interface);
        let mut packet_reader = (self.packet_reader_factory)(Arc::clone(&self.interface));
        let sender = self.sender.clone();

        thread::spawn(move || {
            info!("waiting for packet");
            while let Ok(packet) = packet_reader.next_packet() {
                if let Ok(_) = done.try_recv() {
                    info!("exiting arp packet reader");
                    break;
                }

                let ethernet = &EthernetPacket::new(packet);

                if let Some(ethernet) = ethernet {
                    let header = ArpPacket::new(ethernet.payload());

                    if let Some(header) = header {
                        let op = header.get_operation();
                        let this_mac = interface.mac.unwrap();

                        if op == ArpOperations::Reply && ethernet.get_source() != this_mac {
                            sender
                                .send(ScanMessage::ARPScanResult(ARPScanResult {
                                    hostname: String::from(""),
                                    ip: header.get_sender_proto_addr().to_string(),
                                    mac: ethernet.get_source().to_string(),
                                    vendor: String::from(""),
                                    status: DeviceStatus::Online,
                                }))
                                .unwrap();
                        }
                    }
                }
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
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let mut packet_sender = (self.packet_sender_factory)(Arc::clone(&self.interface));
        let msg_sender = self.sender.clone();

        self.read_packets(done_rx);

        let target_list = targets::ips::new(&self.targets);

        let process_target = |t: String| {
            info!("scanning ARP target: {}", t);
            let target_ipv4 = Ipv4Addr::from_str(&t).unwrap();
            let source_ipv4 = self
                .interface
                .ips
                .iter()
                .find_map(|ip| match ip.ip() {
                    IpAddr::V4(addr) => Some(addr),
                    IpAddr::V6(_) => None,
                })
                .unwrap();

            let mut pkt_buf = [0u8; PKT_ETH_SIZE + PKT_ARP_SIZE];

            // Use scope blocks so we can reborrow our buffer
            {
                // Build our base ethernet frame
                let mut pkt_eth = MutableEthernetPacket::new(&mut pkt_buf).unwrap();

                pkt_eth.set_destination(MacAddr::broadcast());
                pkt_eth.set_source(self.interface.mac.unwrap());
                pkt_eth.set_ethertype(EtherTypes::Arp);
            }

            {
                // Build the ARP frame on top of the ethernet frame
                let mut pkt_arp = MutableArpPacket::new(&mut pkt_buf[PKT_ARP_OFFSET..]).unwrap();

                pkt_arp.set_hardware_type(ArpHardwareTypes::Ethernet);
                pkt_arp.set_protocol_type(EtherTypes::Ipv4);
                pkt_arp.set_hw_addr_len(6);
                pkt_arp.set_proto_addr_len(4);
                pkt_arp.set_operation(ArpOperations::Request);
                pkt_arp.set_sender_hw_addr(self.interface.mac.unwrap());
                pkt_arp.set_sender_proto_addr(source_ipv4);
                pkt_arp.set_target_hw_addr(MacAddr::zero());
                pkt_arp.set_target_proto_addr(target_ipv4);
            }

            // Send to the broadcast address
            packet_sender.send(&pkt_buf).unwrap();

            // Zero buffer for sanity check
            pkt_buf.fill(0);
        };

        target_list.lazy_loop(process_target);

        // TODO make idleTimeout configurable
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(5));
            // run your function here
            done_tx.send(()).unwrap();
            msg_sender.send(ScanMessage::Done(())).unwrap();
        });
    }
}
