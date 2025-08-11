use log::*;
use pnet::util::MacAddr;
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};

use crate::packet::{Sender, build_heartbeat_packet};

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
        if let Ok(mut pkt_sender) = self.packet_sender.lock()
            && let Err(e) = pkt_sender.send(&build_heartbeat_packet(
                self.source_mac,
                self.source_ipv4,
                self.source_port,
            ))
        {
            error!("error sending heartbeat: {}", e);
        }
    }
}

#[cfg(test)]
#[path = "./heartbeat_tests.rs"]
mod tests;
