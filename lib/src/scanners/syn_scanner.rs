//! Provides Scanner implementation for SYN scanning

use log::*;
use pnet::{
    packet::{Packet, ethernet, ip, ipv4, tcp},
    util,
};
use std::{
    io::Error as IOError,
    net,
    str::FromStr,
    sync::{self, Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, Reader, Sender, rst_packet, syn_packet},
    scanners::{ScanError, Scanning, heartbeat::HeartBeat},
    targets::ports::PortTargets,
};

use super::{Device, Port, SYNScanResult, ScanMessage, Scanner};

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
    fn read_packets(&self, done_rx: mpsc::Receiver<()>) -> JoinHandle<Result<(), ScanError>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let heartbeat_packet_sender = Arc::clone(&self.packet_sender);
        let rst_packet_sender = Arc::clone(&self.packet_sender);
        let devices = self.targets.to_owned();
        let notifier = self.notifier.clone();
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let source_port = self.source_port.to_owned();
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

        thread::spawn(move || -> Result<(), ScanError> {
            let mut reader = packet_reader.lock().map_err(|e| ScanError {
                ip: None,
                port: None,
                error: Box::from(e.to_string()),
            })?;

            loop {
                if done_rx.try_recv().is_ok() {
                    debug!("exiting syn packet reader");
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
                let header = ipv4::Ipv4Packet::new(eth.payload());

                if header.is_none() {
                    continue;
                }

                let header = header.unwrap();

                let device_ip = net::IpAddr::V4(header.get_source());
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
                let sequence = tcp_packet.get_sequence();
                let is_syn_ack = flags == tcp::TcpFlags::SYN + tcp::TcpFlags::ACK;
                let is_expected_packet = matches_destination && is_syn_ack;

                if !is_expected_packet {
                    continue;
                }

                let device = devices.iter().find(|&d| d.ip == device_ip.to_string());

                if device.is_none() {
                    continue;
                }

                let device = device.unwrap();

                let port = u16::from_str(&tcp_packet.get_source().to_string());

                if port.is_err() {
                    continue;
                }

                let port = port.unwrap();

                // send rst packet to prevent SYN Flooding
                // https://en.wikipedia.org/wiki/SYN_flood
                // https://security.stackexchange.com/questions/128196/whats-the-advantage-of-sending-an-rst-packet-after-getting-a-response-in-a-syn
                let rst_packet = rst_packet::build(
                    source_mac,
                    source_ipv4,
                    source_port,
                    net::Ipv4Addr::from_str(device.ip.as_str()).unwrap(),
                    util::MacAddr::from_str(device.mac.as_str()).unwrap(),
                    port,
                    sequence + 1,
                );

                let mut rst_sender = rst_packet_sender.lock().map_err(|e| ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(IOError::other(e.to_string())),
                })?;

                debug!("sending RST packet to {}:{}", device.ip, port);

                rst_sender.send(&rst_packet).map_err(|e| ScanError {
                    ip: Some(device.ip.clone()),
                    port: Some(port.to_string()),
                    error: e,
                })?;

                notifier
                    .send(ScanMessage::SYNScanResult(SYNScanResult {
                        device: device.to_owned(),
                        open_port: Port {
                            id: port,
                            service: String::from(""),
                        },
                    }))
                    .map_err(|e| ScanError {
                        ip: Some(device.ip.clone()),
                        port: Some(port.to_string()),
                        error: Box::from(e),
                    })?;
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for SYNScanner
impl Scanner for SYNScanner<'_> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
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
        let idle_timeout = self.idle_timeout.to_owned();
        let source_port = self.source_port.to_owned();

        let read_handle = self.read_packets(done_rx);

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<(), ScanError> {
            let mut scan_error: Option<ScanError> = None;

            let process_port = |port: u16| -> Result<(), ScanError> {
                let mut res: Result<(), ScanError> = Ok(());

                for device in targets.iter() {
                    // throttle packet sending to prevent packet loss
                    thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                    debug!("scanning SYN target: {}:{}", device.ip, port);

                    let dest_ipv4 = net::Ipv4Addr::from_str(&device.ip).map_err(|e| ScanError {
                        ip: Some(device.ip.clone()),
                        port: Some(port.to_string()),
                        error: Box::from(e),
                    });

                    if let Err(scan_error) = dest_ipv4 {
                        res = Err(scan_error);
                        break;
                    }

                    let dest_mac = util::MacAddr::from_str(&device.mac).map_err(|e| ScanError {
                        ip: Some(device.ip.clone()),
                        port: Some(port.to_string()),
                        error: Box::from(e),
                    });

                    if let Err(scan_error) = dest_mac {
                        res = Err(scan_error);
                        break;
                    }

                    let pkt_buf = syn_packet::build(
                        source_mac,
                        source_ipv4,
                        source_port,
                        dest_ipv4.unwrap(),
                        dest_mac.unwrap(),
                        port,
                    );

                    // send info message to consumer
                    let maybe_err = notifier
                        .send(ScanMessage::Info(Scanning {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                        }))
                        .map_err(|e| ScanError {
                            ip: Some(device.ip.clone()),
                            port: Some(port.to_string()),
                            error: Box::from(e),
                        });

                    if maybe_err.is_err() {
                        res = maybe_err;
                        break;
                    }

                    let sender = packet_sender.lock();

                    if sender.is_err() {
                        res = Err(ScanError {
                            ip: None,
                            port: None,
                            error: Box::from("failed to unlock sender"),
                        });
                        break;
                    }

                    // scan device @ port
                    let sent = sender.unwrap().send(&pkt_buf).map_err(|e| ScanError {
                        ip: Some(device.ip.clone()),
                        port: Some(port.to_string()),
                        error: e,
                    });

                    if sent.is_err() {
                        res = sent;
                        break;
                    }
                }

                res
            };

            if let Err(err) = ports.lazy_loop(process_port) {
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
                error: Box::from(IOError::other(
                    "encountered error during syn packet reading",
                )),
            })?;

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
