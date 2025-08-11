//! Provides Scanner implementation for ARP scanning

use log::*;
use pnet::packet::{Packet, arp, ethernet};
use std::{
    io::Error as IOError,
    net,
    sync::{self, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, Reader, Sender, arp_packet},
    scanners::{Device, ScanError, Scanning},
    targets::ips::IPTargets,
};

use super::{ScanMessage, Scanner, heartbeat::HeartBeat};

/// Data structure representing an ARP scanner
pub struct ARPScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Arc<IPTargets>,
    source_port: u16,
    include_vendor: bool,
    include_host_names: bool,
    idle_timeout: Duration,
    notifier: sync::mpsc::Sender<ScanMessage>,
}

/// Data structure holding parameters needed to create instance of ARPScanner
pub struct ARPScannerArgs<'net> {
    /// The network interface to use when scanning
    pub interface: &'net NetworkInterface,
    /// A packet Reader implementation (can use default provided in packet
    /// crate)
    pub packet_reader: Arc<Mutex<dyn Reader>>,
    /// A packet Sender implementation (can use default provided in packet
    /// crate)
    pub packet_sender: Arc<Mutex<dyn Sender>>,
    /// [`IPTargets`] to scan
    pub targets: Arc<IPTargets>,
    /// An open source port to listen for incoming packets (can use network
    /// packet to find open port)
    pub source_port: u16,
    /// Whether or not to include vendor look-ups for detected devices
    pub include_vendor: bool,
    /// Whether or not to include hostname look-ups for detected devices
    pub include_host_names: bool,
    /// The amount of time to wait for incoming packets after scanning all
    /// targets
    pub idle_timeout: Duration,
    /// Channel to send messages regarding devices being scanned, and detected
    /// devices
    pub notifier: sync::mpsc::Sender<ScanMessage>,
}

impl<'net> ARPScanner<'net> {
    /// Returns an instance of ARPScanner
    pub fn new(args: ARPScannerArgs<'net>) -> Self {
        Self {
            interface: args.interface,
            packet_reader: args.packet_reader,
            packet_sender: args.packet_sender,
            targets: args.targets,
            source_port: args.source_port,
            include_vendor: args.include_vendor,
            include_host_names: args.include_host_names,
            idle_timeout: args.idle_timeout,
            notifier: args.notifier,
        }
    }
}

impl ARPScanner<'_> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done: sync::mpsc::Receiver<()>) -> JoinHandle<Result<(), ScanError>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let packet_sender = Arc::clone(&self.packet_sender);
        let include_host_names = self.include_host_names;
        let include_vendor = self.include_vendor;
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let source_port = self.source_port.to_owned();
        let notifier = self.notifier.clone();
        let (heartbeat_tx, heartbeat_rx) = sync::mpsc::channel::<()>();

        // since reading packets off the wire is a blocking operation, we
        // won't be able to detect a "done" signal if no packets are being
        // received as we'll be blocked on waiting for one to come it. To fix
        // this we send periodic "heartbeat" packets so we can continue to
        // check for "done" signals
        thread::spawn(move || {
            debug!("starting arp heartbeat thread");
            let heartbeat = HeartBeat::new(source_mac, source_ipv4, source_port, packet_sender);
            let interval = Duration::from_secs(1);
            loop {
                if heartbeat_rx.try_recv().is_ok() {
                    debug!("stopping arp heartbeat");
                    break;
                }
                debug!("sending arp heartbeat");
                heartbeat.beat();
                thread::sleep(interval);
            }
        });

        thread::spawn(move || -> Result<(), ScanError> {
            let mut reader = packet_reader.lock().map_err(|e| ScanError {
                ip: None,
                port: None,
                error: Box::from(e.to_string()),
            })?;

            loop {
                if done.try_recv().is_ok() {
                    debug!("exiting arp packet reader");
                    if let Err(e) = heartbeat_tx.send(()) {
                        error!("failed to stop heartbeat: {}", e);
                    }
                    break;
                }

                let pkt = reader.next_packet().map_err(|e| ScanError {
                    ip: None,
                    port: None,
                    error: e,
                })?;

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
                        debug!("looking up hostname for {}", ip4);
                        if let Ok(host) = dns_lookup::lookup_addr(&ip4.into()) {
                            hostname = host;
                        }
                    }

                    let mut vendor = String::from("");
                    if include_vendor && let Some(vendor_data) = oui_data::lookup(&mac) {
                        vendor = vendor_data.organization().to_owned();
                    }

                    let _ = notification_sender.send(ScanMessage::ARPScanResult(Device {
                        hostname,
                        ip: ip4.to_string(),
                        mac,
                        vendor,
                        is_current_host: ip4 == source_ipv4,
                    }));
                });
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner for ARPScanner<'_> {
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

                let pkt_buf = arp_packet::build(source_ipv4, source_mac, target_ipv4);

                // inform consumer we are scanning this target (ignore error on failure to notify)
                notifier
                    .send(ScanMessage::Info(Scanning {
                        ip: target_ipv4.to_string(),
                        port: None,
                    }))
                    .map_err(|e| ScanError {
                        ip: Some(target_ipv4.to_string()),
                        port: None,
                        error: Box::from(e),
                    })?;

                let mut pkt_sender = packet_sender.lock().map_err(|e| ScanError {
                    ip: Some(target_ipv4.to_string()),
                    port: None,
                    error: Box::from(IOError::other(e.to_string())),
                })?;

                // Send to the broadcast address
                pkt_sender.send(&pkt_buf).map_err(|e| ScanError {
                    ip: Some(target_ipv4.to_string()),
                    port: None,
                    error: e,
                })?;

                Ok(())
            };

            let mut scan_error: Option<ScanError> = None;

            if let Err(err) = targets.lazy_loop(process_target) {
                scan_error = Some(err);
            }

            thread::sleep(idle_timeout);

            notifier.send(ScanMessage::Done).map_err(|e| ScanError {
                ip: None,
                port: None,
                error: Box::from(e),
            })?;

            // ignore errors here as the thread may already be dead due to error
            // we'll catch any errors from that thread below and report
            let _ = done_tx.send(());

            let read_result = read_handle.join().map_err(|_| ScanError {
                ip: None,
                port: None,
                error: Box::from("error encountered in arp packet reading thread"),
            })?;

            if let Some(err) = scan_error {
                return Err(err);
            }

            read_result
        })
    }
}

#[cfg(test)]
#[path = "./arp_scanner_tests.rs"]
mod tests;
