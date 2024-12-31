use log::*;
use pnet::packet::{arp, ethernet, Packet};
use std::{
    io::{Error as IOError, ErrorKind},
    net,
    sync::{self, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, arp::ARPPacket, Reader, Sender},
    scanners::{Device, ScanError, Scanning},
    targets::ips::IPTargets,
};

use super::{DeviceStatus, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct ARPScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Arc<IPTargets>,
    include_vendor: bool,
    include_host_names: bool,
    idle_timeout: Duration,
    notifier: sync::mpsc::Sender<ScanMessage>,
}

impl<'net> ARPScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader: Arc<Mutex<dyn Reader>>,
        packet_sender: Arc<Mutex<dyn Sender>>,
        targets: Arc<IPTargets>,
        vendor: bool,
        host: bool,
        idle_timeout: Duration,
        notifier: sync::mpsc::Sender<ScanMessage>,
    ) -> Self {
        Self {
            interface,
            packet_reader,
            packet_sender,
            targets,
            include_vendor: vendor,
            include_host_names: host,
            idle_timeout,
            notifier,
        }
    }
}

impl<'net> ARPScanner<'net> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done: sync::mpsc::Receiver<()>) -> JoinHandle<Result<(), ScanError>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let include_host_names = self.include_host_names.clone();
        let include_vendor = self.include_vendor.clone();
        let notifier = self.notifier.clone();

        thread::spawn(move || -> Result<(), ScanError> {
            let mut reader = packet_reader.lock().or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(IOError::new(ErrorKind::Other, e.to_string())),
                })
            })?;

            loop {
                let pkt = reader.next_packet().or_else(|e| {
                    Err(ScanError {
                        ip: None,
                        port: None,
                        error: Box::new(e),
                    })
                })?;

                if let Ok(_) = done.try_recv() {
                    debug!("exiting arp packet reader");
                    break;
                }

                let eth = ethernet::EthernetPacket::new(pkt);

                if eth.is_none() {
                    continue;
                }

                let eth = eth.unwrap();

                let header = arp::ArpPacket::new(eth.payload());

                if header.is_none() {
                    continue;
                }

                let header = header.unwrap();

                let op = header.get_operation();

                // Capture ANY ARP reply as it's an indiction that there's a
                // device on the network
                let is_expected_arp_packet = op == arp::ArpOperations::Reply;

                if !is_expected_arp_packet {
                    continue;
                }

                let ip4 = header.get_sender_proto_addr();
                let mac = eth.get_source().to_string();

                let notification_sender = notifier.clone();

                // use a separate thread here so we don't slow down packet
                // processing
                thread::spawn(move || {
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

                    let _ = notification_sender.send(ScanMessage::ARPScanResult(Device {
                        hostname,
                        ip: ip4.to_string(),
                        mac,
                        status: DeviceStatus::Online,
                        vendor,
                    }));
                });
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
        let notifier = self.notifier.clone();
        let packet_sender = Arc::clone(&self.packet_sender);
        let idle_timeout = self.idle_timeout;
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let targets = Arc::clone(&self.targets);

        let read_handle = self.read_packets(done_rx);

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<(), ScanError> {
            let process_target = |target_ipv4: net::Ipv4Addr| {
                // throttle packet sending to prevent packet loss
                thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                debug!("scanning ARP target: {}", target_ipv4);

                let pkt_buf = ARPPacket::new(source_ipv4, source_mac, target_ipv4);

                // inform consumer we are scanning this target (ignore error on fail to send)
                notifier
                    .send(ScanMessage::Info(Scanning {
                        ip: target_ipv4.to_string(),
                        port: None,
                    }))
                    .or_else(|e| {
                        Err(ScanError {
                            ip: Some(target_ipv4.to_string()),
                            port: None,
                            error: Box::from(e),
                        })
                    })?;

                let mut sender = packet_sender.lock().or_else(|e| {
                    Err(ScanError {
                        ip: Some(target_ipv4.to_string()),
                        port: None,
                        error: Box::from(IOError::new(ErrorKind::Other, e.to_string())),
                    })
                })?;

                // Send to the broadcast address
                sender.send(&pkt_buf).or_else(|e| {
                    Err(ScanError {
                        ip: Some(target_ipv4.to_string()),
                        port: None,
                        error: Box::from(e),
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
                    ip: None,
                    port: None,
                    error: Box::from(e),
                })
            })?;

            notifier.send(ScanMessage::Done(())).or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(e),
                })
            })?;

            let read_result = read_handle.join().or_else(|_| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(IOError::new(
                        ErrorKind::Other,
                        "error encountered in arp packet reading thread",
                    )),
                })
            })?;

            read_result
        })
    }
}

unsafe impl<'net> Sync for ARPScanner<'net> {}
unsafe impl<'net> Send for ARPScanner<'net> {}
