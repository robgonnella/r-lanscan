use log::*;
use pnet::{
    datalink::NetworkInterface,
    packet::{
        arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket},
        ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket},
        icmpv6::ndp::{NdpOptionPacket, NeighborAdvertPacket, NeighborSolicitPacket},
        ipv6::Ipv6Packet,
        Packet,
    },
    util::MacAddr,
};

use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    packet,
    scanners::ARPScanResult,
    targets::{self, LazyLooper},
};

use super::{DeviceStatus, ScanMessage, Scanner};

// Constants used to help locate our nested packets
const PKT_ETH_SIZE: usize = EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = ArpPacket::minimum_packet_size();
const PKT_IP6_SIZE: usize = Ipv6Packet::minimum_packet_size();
const PKT_NDP_SOL_SIZE: usize = NeighborSolicitPacket::minimum_packet_size();
const PKT_NDP_ADV_SIZE: usize = NeighborAdvertPacket::minimum_packet_size();
const PKT_OPT_SIZE: usize = NdpOptionPacket::minimum_packet_size();
const PKT_MAC_SIZE: usize = 6;

const PKT_ARP_OFFSET: usize = PKT_ETH_SIZE;
const PKT_IP6_OFFSET: usize = PKT_ETH_SIZE;
const PKT_NDP_OFFSET: usize = PKT_IP6_OFFSET + PKT_IP6_SIZE;

const PKT_MIN_ARP_RESP_SIZE: usize = PKT_ETH_SIZE + PKT_ARP_SIZE;
const PKT_MIN_NDP_RESP_SIZE: usize = PKT_ETH_SIZE + PKT_IP6_SIZE + PKT_NDP_ADV_SIZE;

// Data structure representing an ARP scanner
pub struct ARPScanner {
    interface: Arc<NetworkInterface>,
    packet_reader: Arc<Mutex<Box<dyn packet::Reader>>>,
    packet_sender: Arc<Mutex<Box<dyn packet::Sender>>>,
    targets: Vec<String>,
    include_vendor: bool,
    include_host_names: bool,
    sender: mpsc::Sender<ScanMessage>,
}

// Returns a new instance of ARPScanner
pub fn new(
    interface: Arc<NetworkInterface>,
    packet_reader: Arc<Mutex<Box<dyn packet::Reader>>>,
    packet_sender: Arc<Mutex<Box<dyn packet::Sender>>>,
    targets: Vec<String>,
    vendor: bool,
    host: bool,
    sender: mpsc::Sender<ScanMessage>,
) -> ARPScanner {
    ARPScanner {
        interface,
        packet_reader,
        packet_sender,
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
        let interface = Arc::clone(&self.interface);
        let reader = Arc::clone(&self.packet_reader);
        let sender_clone = self.sender.clone();

        thread::spawn(move || {
            let mut reader = reader.lock().unwrap();
            while let Ok(packet) = reader.next_packet() {
                let ethernet = &EthernetPacket::new(packet);
                // info!("ethernet: {:?}", ethernet);

                if let Some(ethernet) = ethernet {
                    let header = ArpPacket::new(ethernet.payload());

                    // info!("arp header: {:?}", header);

                    if let Some(header) = header {
                        let op = header.get_operation();
                        let this_mac = interface.mac.unwrap();

                        if op == ArpOperations::Reply && ethernet.get_source() != this_mac {
                            println!(
                                "ARP packet: {}({}) > {}({}); operation: {:?}",
                                ethernet.get_source(),
                                header.get_sender_proto_addr(),
                                ethernet.get_destination(),
                                header.get_target_proto_addr(),
                                header.get_operation()
                            );
                            sender_clone
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

        self.read_packets();

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
            self.packet_sender.lock().unwrap().send(&pkt_buf).unwrap();

            // Zero buffer for sanity check
            pkt_buf.fill(0);
        };

        target_list.lazy_loop(process_target);

        // TODO make idleTimeout configurable
        thread::sleep(Duration::from_secs(5));
        self.sender.send(ScanMessage::Done(())).unwrap();
    }
}
