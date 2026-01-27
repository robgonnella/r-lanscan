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
    let sender: Arc<Mutex<dyn Sender>> =
        Arc::new(Mutex::new(MockPacketSender::new()));

    let heart_beat = HeartBeat::builder()
        .source_mac(source_mac)
        .source_ipv4(source_ip)
        .source_port(source_port)
        .packet_sender(sender)
        .build()
        .unwrap();

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

    let heart_beat = HeartBeat::builder()
        .source_mac(source_mac)
        .source_ipv4(source_ip)
        .source_port(source_port)
        .packet_sender(sender)
        .build()
        .unwrap();

    heart_beat.beat().unwrap();
}

#[test]
fn sends_heartbeat_packets_in_thread() {
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

    let heart_beat = HeartBeat::builder()
        .source_mac(source_mac)
        .source_ipv4(source_ip)
        .source_port(source_port)
        .packet_sender(sender)
        .build()
        .unwrap();

    let (stop_tx, stop_rx) = mpsc::channel();

    let handle = heart_beat.start_in_thread(stop_rx).unwrap();

    thread::sleep(Duration::from_millis(2000));

    stop_tx.send(()).unwrap();

    handle.join().unwrap().unwrap();
}
