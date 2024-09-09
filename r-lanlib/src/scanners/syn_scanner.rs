use log::*;
use pnet::{
    packet::{ethernet, ip, ipv4, tcp, Packet},
    util,
};
use std::{
    net,
    str::FromStr,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, syn::SYNPacket, Reader, Sender},
    scanners::{ScanError, Scanning},
    targets::{ports::PortTargets, LazyLooper},
};

use super::{Device, Port, PortStatus, SYNScanResult, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct SYNScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Vec<Device>,
    ports: Arc<PortTargets>,
    idle_timeout: Duration,
    sender: mpsc::Sender<ScanMessage>,
    source_port: u16,
}

impl<'net> SYNScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader: Arc<Mutex<dyn Reader>>,
        packet_sender: Arc<Mutex<dyn Sender>>,
        targets: Vec<Device>,
        ports: Arc<PortTargets>,
        idle_timeout: Duration,
        sender: mpsc::Sender<ScanMessage>,
        source_port: u16,
    ) -> Self {
        Self {
            interface,
            packet_reader,
            packet_sender,
            targets,
            ports,
            idle_timeout,
            sender,
            source_port,
        }
    }
}

impl<'net> SYNScanner<'net> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done_rx: mpsc::Receiver<()>) -> JoinHandle<Result<(), ScanError>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let devices = self.targets.to_owned();
        let sender = self.sender.clone();
        let source_port = self.source_port.to_owned();

        thread::spawn(move || -> Result<(), ScanError> {
            let mut reader = packet_reader.lock().or_else(|e| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;

            while let Ok(pkt) = reader.next_packet() {
                if let Ok(_) = done_rx.try_recv() {
                    debug!("exiting syn packet reader");
                    break;
                }

                let eth: &Option<ethernet::EthernetPacket> = &ethernet::EthernetPacket::new(pkt);

                if eth.is_none() {
                    continue;
                }

                let eth = eth.as_ref().unwrap();
                let header = ipv4::Ipv4Packet::new(eth.payload());

                if header.is_none() {
                    continue;
                }

                let header = header.unwrap();

                let source = net::IpAddr::V4(header.get_source());
                let protocol = header.get_next_level_protocol();
                let payload = header.payload();

                if protocol != ip::IpNextHeaderProtocols::Tcp {
                    continue;
                }

                let tcp_packet = tcp::TcpPacket::new(payload);

                if tcp_packet.is_none() {
                    continue;
                }

                let tcp_packet = tcp_packet.unwrap();

                let destination_port = tcp_packet.get_destination();
                let matches_destination = destination_port == source_port;
                let flags: u8 = tcp_packet.get_flags();
                let is_syn_ack = flags == tcp::TcpFlags::SYN + tcp::TcpFlags::ACK;
                let is_expected_packet = matches_destination && is_syn_ack;

                if !is_expected_packet {
                    continue;
                }

                let device = devices.iter().find(|&d| d.ip == source.to_string());

                if device.is_none() {
                    continue;
                }

                let device = device.unwrap();

                let port = u16::from_str(&tcp_packet.get_source().to_string());

                if port.is_err() {
                    continue;
                }

                let port = port.unwrap();

                sender
                    .send(ScanMessage::SYNScanResult(SYNScanResult {
                        device: device.to_owned(),
                        open_port: Port {
                            id: port,
                            service: String::from(""),
                            status: PortStatus::Open,
                        },
                    }))
                    .or_else(|e| {
                        Err(ScanError {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                            msg: e.to_string(),
                        })
                    })?;
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for SYNScanner
impl<'net> Scanner for SYNScanner<'net> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
        debug!("performing SYN scan on targets: {:?}", self.targets);

        debug!("starting syn packet reader");

        let (done_tx, done_rx) = mpsc::channel::<()>();
        let msg_sender = self.sender.clone();
        let packet_sender = Arc::clone(&self.packet_sender);
        let targets = self.targets.clone();
        let interface = self.interface;
        let source_ipv4 = interface.ipv4;
        let source_mac = self.interface.mac;
        let ports = Arc::clone(&self.ports);
        let idle_timeout = self.idle_timeout.to_owned();
        let source_port = self.source_port.to_owned();

        let read_handle = self.read_packets(done_rx);

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<(), ScanError> {
            for device in targets.iter() {
                let process_port = |port: u16| -> Result<(), ScanError> {
                    // throttle packet sending to prevent packet loss
                    thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);
                    debug!("scanning SYN target: {}:{}", device.ip, port);

                    let dest_ipv4 = net::Ipv4Addr::from_str(&device.ip).or_else(|e| {
                        Err(ScanError {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                            msg: e.to_string(),
                        })
                    })?;

                    let dest_mac = util::MacAddr::from_str(&device.mac).or_else(|e| {
                        Err(ScanError {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                            msg: e.to_string(),
                        })
                    })?;

                    let pkt_buf = SYNPacket::new(
                        source_mac,
                        source_ipv4,
                        source_port,
                        dest_ipv4,
                        dest_mac,
                        port,
                    );

                    // send info message to consumer
                    msg_sender
                        .send(ScanMessage::Info(Scanning {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                        }))
                        .or_else(|e| {
                            Err(ScanError {
                                ip: device.ip.clone(),
                                port: Some(port.to_string()),
                                msg: e.to_string(),
                            })
                        })?;

                    let mut sender = packet_sender.lock().or_else(|e| {
                        Err(ScanError {
                            ip: "".to_string(),
                            port: None,
                            msg: e.to_string(),
                        })
                    })?;

                    // scan device @ port
                    sender.send(&pkt_buf).or_else(|e| {
                        Err(ScanError {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                            msg: e.to_string(),
                        })
                    })?;

                    Ok(())
                };

                ports.lazy_loop(process_port)?;
            }

            thread::sleep(idle_timeout);

            done_tx.send(()).or_else(|e| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;

            msg_sender.send(ScanMessage::Done(())).or_else(|e| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: None,
                    msg: e.to_string(),
                })
            })?;

            let read_result = read_handle.join().or_else(|_| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: None,
                    msg: "encounterd error during syn packet reading".to_string(),
                })
            })?;

            read_result
        })
    }
}

unsafe impl<'net> Sync for SYNScanner<'net> {}
unsafe impl<'net> Send for SYNScanner<'net> {}
