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

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use super::*;

    use crate::packet::mocks::MockPacketSender;
    use crate::packet::Sender;

    #[test]
    fn new() {
        let source_ip = Ipv4Addr::from_str("192.168.1.1").unwrap();
        let source_mac = MacAddr::default();
        let source_port = 54321;
        let sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(MockPacketSender::new()));

        let heart_beat = HeartBeat::new(source_mac, source_ip, source_port, sender);

        assert_eq!(heart_beat.source_mac, source_mac);
        assert_eq!(heart_beat.source_ipv4, source_ip);
        assert_eq!(heart_beat.source_port, source_port);
    }

    #[test]
    fn sends_heartbeat_packets() {
        let source_ip = Ipv4Addr::from_str("192.168.1.1").unwrap();
        let source_mac = MacAddr::default();
        let source_port = 54321;

        let mut packet_sender = MockPacketSender::new();

        let expected_packet =
            HeartBeatPacket::new(source_mac.clone(), source_ip.clone(), source_port);

        packet_sender
            .expect_send()
            .withf(move |p| p == expected_packet)
            .returning(|_| Ok(()));

        let sender = Arc::new(Mutex::new(packet_sender));

        let heart_beat = HeartBeat::new(source_mac, source_ip, source_port, sender);
        heart_beat.beat();
    }
}
