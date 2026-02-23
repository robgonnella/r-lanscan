//! Provides Scanner implementation for ARP scanning

use derive_builder::Builder;
use pnet::packet::{Packet, arp, ethernet};
use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{self, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use threadpool::ThreadPool;

use crate::{
    error::{RLanLibError, Result},
    network::NetworkInterface,
    packet::{self, arp_packet::ArpPacketBuilder, wire::Wire},
    scanners::{Device, PortSet, Scanning},
    targets::ips::IPTargets,
};

use super::{ScanMessage, Scanner, heartbeat::HeartBeat};

/// Data structure representing an ARP scanner
#[derive(Clone, Builder)]
#[builder(setter(into))]
pub struct ARPScanner {
    /// Network interface to use for scanning
    interface: Arc<NetworkInterface>,
    /// Wire for reading and sending packets on the wire
    wire: Wire,
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
    /// Default gateway IP, used to mark the gateway device in scan results
    #[builder(default)]
    gateway: Option<Ipv4Addr>,
    /// Tracks the send time for each ARP request by target IP
    #[builder(default = "Arc::new(Mutex::new(HashMap::new()))")]
    send_times: Arc<Mutex<HashMap<Ipv4Addr, Instant>>>,
}

impl ARPScanner {
    /// Returns builder for ARPScanner
    pub fn builder() -> ARPScannerBuilder {
        ARPScannerBuilder::default()
    }

    fn process_target(&self, target: Ipv4Addr) -> Result<()> {
        // throttle packet sending to prevent packet loss
        thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

        log::debug!("scanning ARP target: {}", target);

        // The OS never sends an ARP reply to its own IP, so synthesize the
        // device entry immediately rather than waiting for a reply that will
        // never arrive.
        if target == self.interface.ipv4 {
            self.notifier
                .send(ScanMessage::ARPScanDevice(Device {
                    hostname: String::new(),
                    ip: self.interface.ipv4,
                    mac: self.interface.mac,
                    vendor: String::new(),
                    is_current_host: true,
                    is_gateway: self
                        .gateway
                        .is_some_and(|gw| gw == self.interface.ipv4),
                    open_ports: PortSet::new(),
                    latency_ms: Some(0),
                }))
                .map_err(RLanLibError::from_channel_send_error)?;
            return Ok(());
        }

        let arp_packet = ArpPacketBuilder::default()
            .source_ip(self.interface.ipv4)
            .source_mac(self.interface.mac)
            .dest_ip(target)
            .build()?;

        let pkt_buf = arp_packet.to_raw();

        // inform consumer we are scanning this target (ignore error on failure to notify)
        self.notifier
            .send(ScanMessage::Info(Scanning {
                ip: target,
                port: None,
            }))
            .map_err(RLanLibError::from_channel_send_error)?;

        let mut pkt_sender = self.wire.0.lock()?;

        // Record send time immediately before putting the packet on the wire
        if let Ok(mut times) = self.send_times.lock() {
            times.insert(target, Instant::now());
        }

        // Send to the broadcast address
        pkt_sender.send(&pkt_buf)?;

        Ok(())
    }

    fn process_incoming_packet(
        &self,
        pkt: &[u8],
        pool: &ThreadPool,
    ) -> Result<()> {
        let Some(eth) = ethernet::EthernetPacket::new(pkt) else {
            return Ok(());
        };

        let Some(header) = arp::ArpPacket::new(eth.payload()) else {
            return Ok(());
        };

        // Capture ANY ARP reply as it's an indication that there's a
        // device on the network
        if header.get_operation() != arp::ArpOperations::Reply {
            return Ok(());
        }

        let ip4 = header.get_sender_proto_addr();
        let mac = eth.get_source();

        let latency_ms = self.send_times.lock().ok().and_then(|mut times| {
            times.remove(&ip4).map(|t| t.elapsed().as_millis())
        });

        let notification_sender = self.notifier.clone();
        let interface = Arc::clone(&self.interface);
        let include_host_names = self.include_host_names;
        let include_vendor = self.include_vendor;
        let gateway = self.gateway;

        // use a thread pool here so we don't slow down packet
        // processing while limiting concurrent threads
        pool.execute(move || {
            let hostname = if include_host_names {
                log::debug!("looking up hostname for {}", ip4);
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

            let _ =
                notification_sender.send(ScanMessage::ARPScanDevice(Device {
                    hostname,
                    ip: ip4,
                    mac,
                    vendor,
                    is_current_host: ip4 == interface.ipv4,
                    is_gateway: gateway.is_some_and(|gw| gw == ip4),
                    open_ports: PortSet::new(),
                    latency_ms,
                }));
        });

        Ok(())
    }

    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(
        &self,
        done: sync::mpsc::Receiver<()>,
    ) -> Result<JoinHandle<Result<()>>> {
        let (heartbeat_tx, heartbeat_rx) = sync::mpsc::channel::<()>();

        let heartbeat = HeartBeat::builder()
            .source_mac(self.interface.mac)
            .source_ipv4(self.interface.ipv4)
            .source_port(self.source_port)
            .packet_sender(Arc::clone(&self.wire.0))
            .build()?;

        heartbeat.start_in_thread(heartbeat_rx)?;

        let self_clone = self.clone();

        Ok(thread::spawn(move || -> Result<()> {
            let mut reader = self_clone.wire.1.lock()?;
            // Use a bounded thread pool for DNS/vendor lookups to prevent
            // spawning thousands of threads on large networks
            let lookup_pool = ThreadPool::new(8);

            loop {
                if done.try_recv().is_ok() {
                    log::debug!("exiting arp packet reader");
                    if let Err(e) = heartbeat_tx.send(()) {
                        log::error!("failed to stop heartbeat: {}", e);
                    }
                    break;
                }

                let pkt = reader.next_packet()?;

                self_clone.process_incoming_packet(pkt, &lookup_pool)?;
            }

            Ok(())
        }))
    }
}

// Implements the Scanner trait for ARPScanner
impl Scanner for ARPScanner {
    fn scan(&self) -> Result<JoinHandle<Result<()>>> {
        log::debug!("performing ARP scan on targets: {:?}", self.targets);
        log::debug!("include_vendor: {}", self.include_vendor);
        log::debug!("include_host_names: {}", self.include_host_names);
        log::debug!("starting arp packet reader");

        let self_clone = self.clone();
        let (done_tx, done_rx) = sync::mpsc::channel::<()>();

        let read_handle = self.read_packets(done_rx)?;

        // prevent blocking thread so messages can be freely sent to consumer
        let scan_handle = thread::spawn(move || -> Result<()> {
            let mut scan_error: Option<RLanLibError> = None;

            if let Err(err) = self_clone
                .targets
                .lazy_loop(|t| self_clone.process_target(t))
            {
                scan_error = Some(err);
            }

            thread::sleep(self_clone.idle_timeout);

            self_clone
                .notifier
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
