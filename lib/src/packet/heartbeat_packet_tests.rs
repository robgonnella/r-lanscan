use std::str::FromStr;

use util::MacAddr;

use super::*;

#[test]
fn creates_heartbeat_packet() {
    let source_ip = net::Ipv4Addr::from_str("192.168.68.1").unwrap();
    let source_mac = MacAddr::from_str("00:00:00:00:00:00").unwrap();
    let source_port: u16 = 54321;
    let packet = build(source_mac, source_ip, source_port);
    assert!(!packet.is_empty());
}
