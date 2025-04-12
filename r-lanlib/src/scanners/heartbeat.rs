use log::*;
use pnet::util::MacAddr;
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};

use crate::packet::{heartbeat::HeartBeatPacket, Sender};

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
        if let Ok(mut pkt_sender) = self.packet_sender.lock() {
            if let Err(e) = pkt_sender.send(&HeartBeatPacket::new(
                self.source_mac.clone(),
                self.source_ipv4.clone(),
                self.source_port.clone(),
            )) {
                error!("error sending heartbeat: {}", e.to_string());
            }
        }
    }
}
