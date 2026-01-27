//! Provides Heartbeat packet send capabilities to ensure our packet receive
//! loops are continuously processing and capable of evaluating timeouts etc.
//! Otherwise we would be stuck / blocked waiting for incoming packets and
//! unable to determine when idle_timeout has been reached

use derive_builder::Builder;
use log::*;
use pnet::util::MacAddr;
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};

use crate::packet::{Sender, heartbeat_packet::HeartbeatPacketBuilder};

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

    /// Sends a heartbeat packet with provided source info
    pub fn beat(&self) {
        let heartbeat_packet = HeartbeatPacketBuilder::default()
            .source_ip(self.source_ipv4)
            .source_mac(self.source_mac)
            .source_port(self.source_port)
            .build();

        let packet = if let Ok(heartbeat_packet) = heartbeat_packet {
            heartbeat_packet.to_raw()
        } else {
            let err = heartbeat_packet.unwrap_err();
            error!("failed to build heartbeat packet: {}", err);
            return;
        };

        if let Ok(mut pkt_sender) = self.packet_sender.lock() {
            if let Err(err) = pkt_sender.send(&packet) {
                error!("failed to send heartbeat packet: {}", err);
            }
        } else {
            error!("failed to get lock on packet sender to send heartbeat");
        }
    }
}

#[cfg(test)]
#[path = "./heartbeat_tests.rs"]
mod tests;
