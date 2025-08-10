use std::str::FromStr;

use util::MacAddr;

use super::*;

#[test]
fn creates_rst_packet() {
    let source_ip = net::Ipv4Addr::from_str("192.168.68.1").unwrap();
    let source_mac = MacAddr::from_str("00:00:00:00:00:00").unwrap();
    let source_port: u16 = 54321;
    let target_ip = net::Ipv4Addr::from_str("192.168.68.2").unwrap();
    let target_mac = MacAddr::from_str("00:00:00:00:00:01").unwrap();
    let target_port: u16 = 22;
    let packet = RSTPacket::new(
        source_mac,
        source_ip,
        source_port,
        target_ip,
        target_mac,
        target_port,
        1,
    );
    assert!(!packet.is_empty());
}
