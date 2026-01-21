use std::str::FromStr;
use std::sync::Arc;

use super::*;

use crate::packet::Sender;
use crate::packet::mocks::MockPacketSender;

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

    let heartbeat_packet = HeartbeatPacketBuilder::default()
        .source_ip(source_ip)
        .source_mac(source_mac)
        .source_port(source_port)
        .build()
        .unwrap();

    let expected_packet = heartbeat_packet.to_raw();

    packet_sender
        .expect_send()
        .withf(move |p| p == expected_packet)
        .returning(|_| Ok(()));

    let sender = Arc::new(Mutex::new(packet_sender));

    let heart_beat = HeartBeat::new(source_mac, source_ip, source_port, sender);
    heart_beat.beat();
}
