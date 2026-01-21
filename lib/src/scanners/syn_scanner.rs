//! Provides Scanner implementation for SYN scanning

use log::*;
use pnet::packet::{Packet, ethernet, ip, ipv4, tcp};
use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{self, Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    error::{RLanLibError, Result},
    network::NetworkInterface,
    packet::{self, Reader, Sender, rst_packet, syn_packet},
    scanners::{PortSet, Scanning, heartbeat::HeartBeat},
    targets::ports::PortTargets,
};

use super::{Device, Port, ScanMessage, Scanner};

/// Data structure representing an ARP scanner
pub struct SYNScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Vec<Device>,
    ports: Arc<PortTargets>,
    source_port: u16,
    idle_timeout: Duration,
    notifier: mpsc::Sender<ScanMessage>,
}

/// Data structure holding parameters needed to create instance of SYNScanner
pub struct SYNScannerArgs<'net> {
    /// The network interface to use when scanning
    pub interface: &'net NetworkInterface,
    /// A packet Reader implementation (can use default provided in packet
    /// crate)
    pub packet_reader: Arc<Mutex<dyn Reader>>,
    /// A packet Sender implementation (can use default provided in packet
    /// crate)
    pub packet_sender: Arc<Mutex<dyn Sender>>,
    /// [`Device`] list to scan
    pub targets: Vec<Device>,
    /// [`PortTargets`] to scan for each detected device
    pub ports: Arc<PortTargets>,
    /// An open source port to listen for incoming packets (can use network
    /// packet to find open port)
    pub source_port: u16,
    /// The amount of time to wait for incoming packets after scanning all
    /// targets
    pub idle_timeout: Duration,
    /// Channel to send messages regarding devices being scanned, and detected
    /// devices
    pub notifier: mpsc::Sender<ScanMessage>,
}

impl<'net> SYNScanner<'net> {
    /// Returns a new instance of SYNScanner using provided info
    pub fn new(args: SYNScannerArgs<'net>) -> Self {
        Self {
            interface: args.interface,
            packet_reader: args.packet_reader,
            packet_sender: args.packet_sender,
            targets: args.targets,
            ports: args.ports,
            source_port: args.source_port,
            idle_timeout: args.idle_timeout,
            notifier: args.notifier,
        }
    }
}

impl SYNScanner<'_> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done_rx: mpsc::Receiver<()>) -> JoinHandle<Result<()>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let heartbeat_packet_sender = Arc::clone(&self.packet_sender);
        let rst_packet_sender = Arc::clone(&self.packet_sender);
        // Build a HashMap for O(1) device lookups instead of O(n) linear search
        let device_map: HashMap<Ipv4Addr, Device> =
            self.targets.iter().map(|d| (d.ip, d.clone())).collect();
        let notifier = self.notifier.clone();
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let source_port = self.source_port;
        let (heartbeat_tx, heartbeat_rx) = sync::mpsc::channel::<()>();

        // since reading packets off the wire is a blocking operation, we
        // won't be able to detect a "done" signal if no packets are being
        // received as we'll be blocked on waiting for one to come it. To fix
        // this we send periodic "heartbeat" packets so we can continue to
        // check for "done" signals
        thread::spawn(move || {
            debug!("starting syn heartbeat thread");
            let heartbeat = HeartBeat::new(
                source_mac,
                source_ipv4,
                source_port,
                heartbeat_packet_sender,
            );
            let interval = Duration::from_secs(1);
            loop {
                if heartbeat_rx.try_recv().is_ok() {
                    debug!("stopping syn heartbeat");
                    break;
                }
                debug!("sending syn heartbeat");
                heartbeat.beat();
                thread::sleep(interval);
            }
        });

        thread::spawn(move || -> Result<()> {
            let mut reader = packet_reader.lock()?;

            loop {
                if done_rx.try_recv().is_ok() {
                    debug!("exiting syn packet reader");
                    if let Err(e) = heartbeat_tx.send(()) {
                        error!("failed to stop heartbeat: {}", e);
                    }

                    break;
                }

                let pkt = reader.next_packet()?;

                let Some(eth) = ethernet::EthernetPacket::new(pkt) else {
                    continue;
                };

                let Some(header) = ipv4::Ipv4Packet::new(eth.payload()) else {
                    continue;
                };

                let device_ip = header.get_source();
                let protocol = header.get_next_level_protocol();
                let payload = header.payload();

                if protocol != ip::IpNextHeaderProtocols::Tcp {
                    continue;
                }

                let Some(tcp_packet) = tcp::TcpPacket::new(payload) else {
                    continue;
                };

                let destination_port = tcp_packet.get_destination();
                let matches_destination = destination_port == source_port;
                let flags: u8 = tcp_packet.get_flags();
                let sequence = tcp_packet.get_sequence();
                let is_syn_ack = flags == tcp::TcpFlags::SYN + tcp::TcpFlags::ACK;

                if !matches_destination || !is_syn_ack {
                    continue;
                }

                let Some(device) = device_map.get(&device_ip) else {
                    continue;
                };

                let port = tcp_packet.get_source();

                // send rst packet to prevent SYN Flooding
                // https://en.wikipedia.org/wiki/SYN_flood
                // https://security.stackexchange.com/questions/128196/whats-the-advantage-of-sending-an-rst-packet-after-getting-a-response-in-a-syn
                let dest_ipv4 = device.ip;
                let dest_mac = device.mac;

                let rst_packet = rst_packet::build(
                    source_mac,
                    source_ipv4,
                    source_port,
                    dest_ipv4,
                    dest_mac,
                    port,
                    sequence + 1,
                );

                let mut rst_sender = rst_packet_sender.lock()?;

                debug!("sending RST packet to {}:{}", device.ip, port);

                rst_sender.send(&rst_packet)?;

                let mut ports = PortSet::new();
                ports.0.insert(Port {
                    id: port,
                    service: "".into(),
                });

                notifier
                    .send(ScanMessage::SYNScanDevice(Device {
                        open_ports: ports,
                        ..device.clone()
                    }))
                    .map_err(RLanLibError::from_channel_send_error)?;
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for SYNScanner
impl Scanner for SYNScanner<'_> {
    fn scan(&self) -> JoinHandle<Result<()>> {
        debug!("performing SYN scan on targets: {:?}", self.targets);

        debug!("starting syn packet reader");

        let (done_tx, done_rx) = mpsc::channel::<()>();
        let notifier = self.notifier.clone();
        let packet_sender = Arc::clone(&self.packet_sender);
        let targets = self.targets.clone();
        let interface = self.interface;
        let source_ipv4 = interface.ipv4;
        let source_mac = self.interface.mac;
        let ports = Arc::clone(&self.ports);
        let idle_timeout = self.idle_timeout;
        let source_port = self.source_port;

        let read_handle = self.read_packets(done_rx);

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<()> {
            let mut scan_error: Option<RLanLibError> = None;

            let process_port = |port: u16| -> Result<()> {
                for device in targets.iter() {
                    // throttle packet sending to prevent packet loss
                    thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                    debug!("scanning SYN target: {}:{}", device.ip, port);

                    let dest_ipv4 = device.ip;
                    let dest_mac = device.mac;

                    let pkt_buf = syn_packet::build(
                        source_mac,
                        source_ipv4,
                        source_port,
                        dest_ipv4,
                        dest_mac,
                        port,
                    );

                    // send info message to consumer
                    notifier
                        .send(ScanMessage::Info(Scanning {
                            ip: device.ip,
                            port: Some(port),
                        }))
                        .map_err(RLanLibError::from_channel_send_error)?;

                    let mut sender = packet_sender.lock()?;

                    // scan device @ port
                    sender.send(&pkt_buf).map_err(|e| RLanLibError::Scan {
                        ip: Some(device.ip.to_string()),
                        port: Some(port.to_string()),
                        error: e.to_string(),
                    })?;
                }

                Ok(())
            };

            if let Err(err) = ports.lazy_loop(process_port) {
                scan_error = Some(err);
            }

            thread::sleep(idle_timeout);

            notifier
                .send(ScanMessage::Done)
                .map_err(RLanLibError::from_channel_send_error)?;

            // ignore errors here as the thread may already be dead due to error
            // we'll catch any errors from that thread below and report
            let _ = done_tx.send(());

            let read_result = read_handle.join()?;

            if let Some(err) = scan_error {
                return Err(err);
            }

            read_result
        })
    }
}

#[cfg(test)]
#[path = "./syn_scanner_tests.rs"]
mod tests;
