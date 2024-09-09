use log::*;
use pnet::{
    packet::{arp, ethernet, Packet},
    util::MacAddr,
};
use std::{
    net,
    str::FromStr,
    sync::{self, Arc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, arp::ARPPacket, PacketReaderFactory, PacketSenderFactory},
    scanners::{Device, ScanError, Scanning},
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
    fn read_packets(
        &self,
        done: sync::mpsc::Receiver<()>,
        source_mac: MacAddr,
    ) -> JoinHandle<Result<(), ScanError>> {
        let mut packet_reader = (self.packet_reader_factory)(&self.interface);
        let include_host_names = self.include_host_names.clone();
        let include_vendor = self.include_vendor.clone();
        let sender = self.sender.clone();

        thread::spawn(move || -> Result<(), ScanError> {
            while let Ok(pkt) = packet_reader.next_packet() {
                if let Ok(_) = done.try_recv() {
                    debug!("exiting arp packet reader");
                    break;
                }

                let eth = &ethernet::EthernetPacket::new(pkt);

                if eth.is_none() {
                    continue;
                }

                let eth = eth.as_ref().unwrap();

                let header = arp::ArpPacket::new(eth.payload());

                if header.is_none() {
                    continue;
                }

                let header = header.unwrap();

                let op = header.get_operation();

                let is_expected_arp_packet =
                    op == arp::ArpOperations::Reply && eth.get_source() != source_mac;

                if !is_expected_arp_packet {
                    continue;
                }

                let ip4 = header.get_sender_proto_addr();
                let mac = eth.get_source().to_string();

                let mut hostname: String = String::from("");
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
                    .or_else(|e| {
                        Err(ScanError {
                            ip: ip4.to_string(),
                            port: None,
                            msg: e.to_string(),
                        })
                    })?;
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for ARPScanner
impl<'net> Scanner for ARPScanner<'net> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
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

        let read_handle = self.read_packets(done_rx, source_mac.clone());

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<(), ScanError> {
            let process_target = |t: String| {
                // throttle packet sending to prevent packet loss
                thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);
                debug!("scanning ARP target: {}", t);
                let target_ipv4 = net::Ipv4Addr::from_str(&t).or_else(|e| {
                    Err(ScanError {
                        ip: t.to_string(),
                        port: None,
                        msg: e.to_string(),
                    })
                })?;

                let pkt_buf = ARPPacket::new(source_ipv4, source_mac, target_ipv4);

                // inform consumer we are scanning this target (ignore error on fail to send)
                msg_sender
                    .send(ScanMessage::Info(Scanning {
                        ip: t.to_string(),
                        port: None,
                    }))
                    .or_else(|e| {
                        Err(ScanError {
                            ip: t.to_string(),
                            port: None,
                            msg: e.to_string(),
                        })
                    })?;

                // Send to the broadcast address
                packet_sender.send(&pkt_buf).or_else(|e| {
                    Err(ScanError {
                        ip: t.to_string(),
                        port: None,
                        msg: e.to_string(),
                    })
                })?;

                Ok(())
            };

            if let Err(err) = targets.lazy_loop(process_target) {
                return Err(err);
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
                    msg: "error encountered in arp packet reading thread".to_string(),
                })
            })?;

            read_result
        })
    }
}

unsafe impl<'net> Sync for ARPScanner<'net> {}
unsafe impl<'net> Send for ARPScanner<'net> {}
