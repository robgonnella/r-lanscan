use std::str::FromStr;

use util::MacAddr;

use super::*;

#[test]
fn creates_heartbeat_packet() {
    let source_ip = net::Ipv4Addr::from_str("192.168.68.1").unwrap();
    let source_mac = MacAddr::from_str("00:00:00:00:00:00").unwrap();
    let source_port: u16 = 54321;
    let heartbeat_packet = HeartbeatPacketBuilder::default()
        .source_ip(source_ip)
        .source_mac(source_mac)
        .source_port(source_port)
        .build()
        .unwrap();
    let packet = heartbeat_packet.to_raw();
    assert!(!packet.is_empty());
}
