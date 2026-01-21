use log::*;
use pnet::util::MacAddr;
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};

use crate::packet::{Sender, heartbeat_packet::HeartbeatPacketBuilder};

pub struct HeartBeat {
    source_mac: MacAddr,
    source_ipv4: Ipv4Addr,
    source_port: u16,
    packet_sender: Arc<Mutex<dyn Sender>>,
}

impl HeartBeat {
    pub fn new(
        source_mac: MacAddr,
        source_ipv4: Ipv4Addr,
        source_port: u16,
        packet_sender: Arc<Mutex<dyn Sender>>,
    ) -> Self {
        Self {
            source_mac,
            source_ipv4,
            source_port,
            packet_sender,
        }
    }

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
