//! Provides Scanner implementation for SYN scanning

use derive_builder::Builder;
use log::*;
use pnet::packet::{Packet, ethernet, ip, ipv4, tcp};
use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{self, Arc, LazyLock, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    error::{RLanLibError, Result},
    network::NetworkInterface,
    packet::{
        self, Reader, Sender, rst_packet::RstPacketBuilder,
        syn_packet::SynPacketBuilder,
    },
    scanners::{PortSet, Scanning, heartbeat::HeartBeat},
    targets::ports::PortTargets,
};

use super::{Device, Port, ScanMessage, Scanner};

static SERVICES: LazyLock<HashMap<u16, &str>> = LazyLock::new(|| {
    HashMap::from([
        (20, "ftp-data"),
        (21, "ftp"),
        (22, "ssh"),
        (23, "telnet"),
        (25, "smtp"),
        (53, "dns"),
        (80, "http"),
        (110, "pop3"),
        (143, "imap"),
        (443, "https"),
        (445, "microsoft-ds"),
        (587, "submission"),
        (993, "imaps"),
        (995, "pop3s"),
        (1433, "mssql"),
        (3306, "mysql"),
        (3389, "rdp"),
        (5432, "postgresql"),
        (5900, "vnc"),
        (6379, "redis"),
        (8080, "http-alt"),
        (8443, "https-alt"),
        (27017, "mongodb"),
    ])
});

/// Data structure representing a SYN scanner
#[derive(Builder)]
#[builder(setter(into))]
pub struct SYNScanner<'net> {
    /// Network interface to use for scanning
    interface: &'net NetworkInterface,
    /// Packet reader for receiving SYN-ACK responses
    packet_reader: Arc<Mutex<dyn Reader>>,
    /// Packet sender for transmitting SYN packets
    packet_sender: Arc<Mutex<dyn Sender>>,
    /// Devices to scan for open ports
    targets: Vec<Device>,
    /// Port targets to scan on each device
    ports: Arc<PortTargets>,
    /// Source port for packet listener and incoming packet identification
    source_port: u16,
    /// Duration to wait for responses after scanning completes
    idle_timeout: Duration,
    /// Channel for sending scan results and status messages
    notifier: mpsc::Sender<ScanMessage>,
}

impl<'n> SYNScanner<'n> {
    /// Returns a builder for SYNScanner
    pub fn builder() -> SYNScannerBuilder<'n> {
        SYNScannerBuilder::default()
    }

    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(
        &self,
        done_rx: mpsc::Receiver<()>,
    ) -> Result<JoinHandle<Result<()>>> {
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

        let heartbeat = HeartBeat::builder()
            .source_mac(source_mac)
            .source_ipv4(source_ipv4)
            .source_port(source_port)
            .packet_sender(heartbeat_packet_sender)
            .build()?;

        heartbeat.start_in_thread(heartbeat_rx)?;

        Ok(thread::spawn(move || -> Result<()> {
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
                let is_syn_ack =
                    flags == tcp::TcpFlags::SYN + tcp::TcpFlags::ACK;

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

                let rst_packet = RstPacketBuilder::default()
                    .source_ip(source_ipv4)
                    .source_mac(source_mac)
                    .source_port(source_port)
                    .dest_ip(dest_ipv4)
                    .dest_mac(dest_mac)
                    .dest_port(port)
                    .sequence_number(sequence + 1)
                    .build()?;

                let rst_packet = rst_packet.to_raw();

                let mut rst_sender = rst_packet_sender.lock()?;

                debug!("sending RST packet to {}:{}", device.ip, port);

                rst_sender.send(&rst_packet)?;

                let service = SERVICES
                    .get(&port)
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let mut ports = PortSet::new();
                ports.0.insert(Port { id: port, service });

                notifier
                    .send(ScanMessage::SYNScanDevice(Device {
                        open_ports: ports,
                        ..device.clone()
                    }))
                    .map_err(RLanLibError::from_channel_send_error)?;
            }

            Ok(())
        }))
    }
}

// Implements the Scanner trait for SYNScanner
impl Scanner for SYNScanner<'_> {
    fn scan(&self) -> Result<JoinHandle<Result<()>>> {
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

        let read_handle = self.read_packets(done_rx)?;

        // prevent blocking thread so messages can be freely sent to consumer
        let handle = thread::spawn(move || -> Result<()> {
            let mut scan_error: Option<RLanLibError> = None;

            let process_port = |port: u16| -> Result<()> {
                for device in targets.iter() {
                    // throttle packet sending to prevent packet loss
                    thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                    debug!("scanning SYN target: {}:{}", device.ip, port);

                    let dest_ipv4 = device.ip;
                    let dest_mac = device.mac;

                    let syn_packet = SynPacketBuilder::default()
                        .source_ip(source_ipv4)
                        .source_mac(source_mac)
                        .source_port(source_port)
                        .dest_ip(dest_ipv4)
                        .dest_mac(dest_mac)
                        .dest_port(port)
                        .build()?;

                    let pkt_buf = syn_packet.to_raw();

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
        });

        Ok(handle)
    }
}

#[cfg(test)]
#[path = "./syn_scanner_tests.rs"]
mod tests;
