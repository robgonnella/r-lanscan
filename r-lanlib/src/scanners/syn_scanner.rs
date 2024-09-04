use log::*;
use pnet::{
    datalink,
    packet::{ethernet, ip, ipv4, tcp, Packet},
    util,
};

use core::time;
use std::{net, str::FromStr, sync, thread, time::Duration};

use crate::{
    network, packet,
    targets::{self, LazyLooper},
};

use super::{Device, Port, PortStatus, SYNScanResult, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct SYNScanner {
    interface: sync::Arc<datalink::NetworkInterface>,
    packet_reader_factory: packet::PacketReaderFactory,
    packet_sender_factory: packet::PacketSenderFactory,
    targets: sync::Arc<Vec<Device>>,
    ports: sync::Arc<targets::ports::PortTargets>,
    idle_timeout: Duration,
    sender: sync::mpsc::Sender<ScanMessage>,
    source_port: u16,
}

// Returns a new instance of SYNScanner
pub fn new(
    interface: sync::Arc<datalink::NetworkInterface>,
    packet_reader_factory: packet::PacketReaderFactory,
    packet_sender_factory: packet::PacketSenderFactory,
    targets: sync::Arc<Vec<Device>>,
    ports: sync::Arc<targets::ports::PortTargets>,
    idle_timeout: Duration,
    sender: sync::mpsc::Sender<ScanMessage>,
    source_port: u16,
) -> SYNScanner {
    SYNScanner {
        interface,
        packet_reader_factory,
        packet_sender_factory,
        targets,
        ports,
        idle_timeout,
        sender,
        source_port,
    }
}

impl SYNScanner {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done_rx: sync::mpsc::Receiver<()>) {
        let mut packet_reader = (self.packet_reader_factory)(sync::Arc::clone(&self.interface));
        let devices = self.targets.to_owned();
        let sender = self.sender.clone();
        let source_port = self.source_port.to_owned();

        thread::spawn(move || {
            while let Ok(pkt) = packet_reader.next_packet() {
                if let Ok(_) = done_rx.try_recv() {
                    debug!("exiting syn packet reader");
                    break;
                }

                let eth: &Option<ethernet::EthernetPacket> = &ethernet::EthernetPacket::new(pkt);

                if let Some(eth) = eth {
                    let header = ipv4::Ipv4Packet::new(eth.payload());
                    if let Some(header) = header {
                        let source = net::IpAddr::V4(header.get_source());
                        let protocol = header.get_next_level_protocol();
                        let payload = header.payload();

                        match protocol {
                            ip::IpNextHeaderProtocols::Tcp => {
                                let tcp_packet = tcp::TcpPacket::new(payload);
                                if let Some(tcp_packet) = tcp_packet {
                                    let destination_port = tcp_packet.get_destination();
                                    let device =
                                        devices.iter().find(|&d| d.ip == source.to_string());
                                    let matches_destination = destination_port == source_port;
                                    let flags: u8 = tcp_packet.get_flags();
                                    let is_syn_ack =
                                        flags == tcp::TcpFlags::SYN + tcp::TcpFlags::ACK;

                                    match device {
                                        Some(device) => {
                                            if matches_destination && is_syn_ack {
                                                sender
                                                    .send(ScanMessage::SYNScanResult(
                                                        SYNScanResult {
                                                            device: device.to_owned(),
                                                            open_port: Port {
                                                                id: u16::from_str(
                                                                    &tcp_packet
                                                                        .get_source()
                                                                        .to_string(),
                                                                )
                                                                .unwrap(),
                                                                service: String::from(""),
                                                                status: PortStatus::Open,
                                                            },
                                                        },
                                                    ))
                                                    .unwrap();
                                            }
                                        }
                                        None => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }
}

// Implements the Scanner trait for SYNScanner
impl Scanner for SYNScanner {
    fn scan(&self) {
        debug!("performing SYN scan on targets: {:?}", self.targets);

        debug!("starting syn packet reader");

        let (done_tx, done_rx) = sync::mpsc::channel::<()>();
        let msg_sender = self.sender.clone();
        let mut packet_sender = (self.packet_sender_factory)(sync::Arc::clone(&self.interface));
        let targets = sync::Arc::clone(&self.targets);
        let interface = sync::Arc::clone(&self.interface);
        let source_ipv4 = net::Ipv4Addr::from_str(&network::get_interface_ipv4(interface)).unwrap();
        let source_mac = self.interface.mac.unwrap();
        let ports = sync::Arc::clone(&self.ports);
        let idle_timeout = self.idle_timeout.to_owned();
        let source_port = self.source_port.to_owned();

        self.read_packets(done_rx);

        thread::spawn(move || {
            for device in targets.iter() {
                let process_port = |port: u16| {
                    thread::sleep(time::Duration::from_micros(100));
                    debug!("scanning SYN target: {}:{}", device.ip, port);

                    let ipv4_destination = net::Ipv4Addr::from_str(&device.ip);

                    if ipv4_destination.is_ok() {
                        let source_ipv4 = source_ipv4;
                        let source_mac = source_mac;
                        let dest_ipv4 = ipv4_destination.unwrap();
                        let dest_mac = util::MacAddr::from_str(&device.mac).unwrap();
                        let pkt_buf = packet::syn::new(
                            source_mac,
                            source_ipv4,
                            source_port,
                            dest_ipv4,
                            dest_mac,
                            port,
                        );
                        packet_sender.send(&pkt_buf).unwrap();
                    }
                };

                ports.lazy_loop(process_port)
            }

            thread::sleep(idle_timeout);
            done_tx.send(()).unwrap();
            msg_sender.send(ScanMessage::Done(())).unwrap();
        });
    }
}

unsafe impl Sync for SYNScanner {}
unsafe impl Send for SYNScanner {}
