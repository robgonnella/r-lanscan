//! Provides Heartbeat packet send capabilities to ensure our packet receive
//! loops are continuously processing and capable of evaluating timeouts etc.
//! Otherwise we would be stuck / blocked waiting for incoming packets and
//! unable to determine when idle_timeout has been reached

use derive_builder::Builder;
use pnet::util::MacAddr;
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    error::Result,
    packet::{Sender, heartbeat_packet::HeartbeatPacketBuilder},
};

/// Sends heartbeat packets to ensure we continuously evaluate packet reader
/// channels
#[derive(Builder)]
pub struct HeartBeat {
    /// MAC address to use as source in heartbeat packets
    source_mac: MacAddr,
    /// IPv4 address to use as source in heartbeat packets
    source_ipv4: Ipv4Addr,
    /// Port to use as source in heartbeat packets
    source_port: u16,
    /// Packet sender for transmitting heartbeat packets
    packet_sender: Arc<Mutex<dyn Sender>>,
}

impl HeartBeat {
    /// Returns a builder for Heartbeat
    pub fn builder() -> HeartBeatBuilder {
        HeartBeatBuilder::default()
    }

    /// Sends a single heartbeat packet
    pub fn beat(&self) -> Result<()> {
        let heartbeat_packet = HeartbeatPacketBuilder::default()
            .source_ip(self.source_ipv4)
            .source_mac(self.source_mac)
            .source_port(self.source_port)
            .build()?;

        let packet = heartbeat_packet.to_raw();

        let mut sender = self.packet_sender.lock()?;
        sender.send(&packet)?;
        Ok(())
    }

    /// Starts a separate thread to send heartbeat packets on a 1s interval
    pub fn start_in_thread(
        &self,
        done: mpsc::Receiver<()>,
    ) -> Result<JoinHandle<Result<()>>> {
        // since reading packets off the wire is a blocking operation, we
        // won't be able to detect a "done" signal if no packets are being
        // received as we'll be blocked on waiting for one to come in. To fix
        // this we send periodic "heartbeat" packets so we can continue to
        // check for "done" signals
        let source_mac = self.source_mac;
        let source_ipv4 = self.source_ipv4;
        let source_port = self.source_port;
        let packet_sender = Arc::clone(&self.packet_sender);
        let heartbeat_packet = HeartbeatPacketBuilder::default()
            .source_ip(source_ipv4)
            .source_mac(source_mac)
            .source_port(source_port)
            .build()?;
        let packet = heartbeat_packet.to_raw();

        Ok(thread::spawn(move || -> Result<()> {
            log::debug!("starting heartbeat thread");

            let mut misses = 0;
            let mut send_err = None;
            let interval = Duration::from_secs(1);

            loop {
                if misses >= 5
                    && let Some(err) = send_err.take()
                {
                    return Err(err);
                }
                if done.try_recv().is_ok() {
                    log::debug!("stopping heartbeat");
                    return Ok(());
                }
                log::debug!("sending heartbeat");
                {
                    // scoped to drop lock before end of loop
                    let mut sender = packet_sender.lock()?;
                    if let Err(err) = sender.send(&packet) {
                        misses += 1;
                        send_err = Some(err);
                    } else {
                        misses = 0;
                        send_err = None;
                    }
                }
                thread::sleep(interval);
            }
        }))
    }
}

#[cfg(test)]
#[path = "./heartbeat_tests.rs"]
mod tests;
