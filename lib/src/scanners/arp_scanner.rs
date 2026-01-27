//! Provides Scanner implementation for ARP scanning

use derive_builder::Builder;
use log::*;
use pnet::packet::{Packet, arp, ethernet};
use std::{
    net,
    sync::{self, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};
use threadpool::ThreadPool;

use crate::{
    error::{RLanLibError, Result},
    network::NetworkInterface,
    packet::{self, Reader, Sender, arp_packet::ArpPacketBuilder},
    scanners::{Device, PortSet, Scanning},
    targets::ips::IPTargets,
};

use super::{ScanMessage, Scanner, heartbeat::HeartBeat};

/// Data structure representing an ARP scanner
#[derive(Builder)]
#[builder(setter(into))]
pub struct ARPScanner<'net> {
    /// Network interface to use for scanning
    interface: &'net NetworkInterface,
    /// Packet reader for receiving ARP replies
    packet_reader: Arc<Mutex<dyn Reader>>,
    /// Packet sender for transmitting ARP requests
    packet_sender: Arc<Mutex<dyn Sender>>,
    /// IP targets to scan
    targets: Arc<IPTargets>,
    /// Source port for packet listener and incoming packet identification
    source_port: u16,
    /// Whether to include vendor lookups for discovered devices
    include_vendor: bool,
    /// Whether to include hostname lookups for discovered devices
    include_host_names: bool,
    /// Duration to wait for responses after scanning completes
    idle_timeout: Duration,
    /// Channel for sending scan results and status messages
    notifier: sync::mpsc::Sender<ScanMessage>,
}

impl<'n> ARPScanner<'n> {
    /// Returns builder for ARPScanner
    pub fn builder() -> ARPScannerBuilder<'n> {
        ARPScannerBuilder::default()
    }

    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(
        &self,
        done: sync::mpsc::Receiver<()>,
    ) -> JoinHandle<Result<()>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let packet_sender = Arc::clone(&self.packet_sender);
        let include_host_names = self.include_host_names;
        let include_vendor = self.include_vendor;
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let source_port = self.source_port;
        let notifier = self.notifier.clone();
        let (heartbeat_tx, heartbeat_rx) = sync::mpsc::channel::<()>();

        // since reading packets off the wire is a blocking operation, we
        // won't be able to detect a "done" signal if no packets are being
        // received as we'll be blocked on waiting for one to come it. To fix
        // this we send periodic "heartbeat" packets so we can continue to
        // check for "done" signals
        thread::spawn(move || -> Result<()> {
            debug!("starting arp heartbeat thread");
            let heartbeat = HeartBeat::builder()
                .source_mac(source_mac)
                .source_ipv4(source_ipv4)
                .source_port(source_port)
                .packet_sender(packet_sender)
                .build()?;

            let interval = Duration::from_secs(1);
            loop {
                if heartbeat_rx.try_recv().is_ok() {
                    debug!("stopping arp heartbeat");
                    return Ok(());
                }
                debug!("sending arp heartbeat");
                heartbeat.beat();
                thread::sleep(interval);
            }
        });

        thread::spawn(move || -> Result<()> {
            let mut reader = packet_reader.lock()?;
            // Use a bounded thread pool for DNS/vendor lookups to prevent
            // spawning thousands of threads on large networks
            let lookup_pool = ThreadPool::new(8);

            loop {
                if done.try_recv().is_ok() {
                    debug!("exiting arp packet reader");
                    if let Err(e) = heartbeat_tx.send(()) {
                        error!("failed to stop heartbeat: {}", e);
                    }
                    break;
                }

                let pkt = reader.next_packet()?;

                let Some(eth) = ethernet::EthernetPacket::new(pkt) else {
                    continue;
                };

                let Some(header) = arp::ArpPacket::new(eth.payload()) else {
                    continue;
                };

                // Capture ANY ARP reply as it's an indication that there's a
                // device on the network
                if header.get_operation() != arp::ArpOperations::Reply {
                    continue;
                }

                let ip4 = header.get_sender_proto_addr();
                let mac = eth.get_source();

                let notification_sender = notifier.clone();

                // use a thread pool here so we don't slow down packet
                // processing while limiting concurrent threads
                lookup_pool.execute(move || {
                    let hostname = if include_host_names {
                        debug!("looking up hostname for {}", ip4);
                        dns_lookup::lookup_addr(&ip4.into()).unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let vendor = if include_vendor {
                        oui_data::lookup(&mac.to_string())
                            .map(|v| v.organization().to_owned())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let _ = notification_sender.send(
                        ScanMessage::ARPScanDevice(Device {
                            hostname,
                            ip: ip4,
                            mac,
                            vendor,
                            is_current_host: ip4 == source_ipv4,
                            open_ports: PortSet::new(),
                        }),
                    );
                });
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner for ARPScanner<'_> {
    fn scan(&self) -> Result<JoinHandle<Result<()>>> {
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
        let scan_handle = thread::spawn(move || -> Result<()> {
            let process_target = |target_ipv4: net::Ipv4Addr| {
                // throttle packet sending to prevent packet loss
                thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                debug!("scanning ARP target: {}", target_ipv4);

                let arp_packet = ArpPacketBuilder::default()
                    .source_ip(source_ipv4)
                    .source_mac(source_mac)
                    .dest_ip(target_ipv4)
                    .build()?;

                let pkt_buf = arp_packet.to_raw();

                // inform consumer we are scanning this target (ignore error on failure to notify)
                notifier
                    .send(ScanMessage::Info(Scanning {
                        ip: target_ipv4,
                        port: None,
                    }))
                    .map_err(RLanLibError::from_channel_send_error)?;

                let mut pkt_sender = packet_sender.lock()?;

                // Send to the broadcast address
                pkt_sender.send(&pkt_buf)?;

                Ok(())
            };

            let mut scan_error: Option<RLanLibError> = None;

            if let Err(err) = targets.lazy_loop(process_target) {
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

        Ok(scan_handle)
    }
}

#[cfg(test)]
#[path = "./arp_scanner_tests.rs"]
mod tests;
