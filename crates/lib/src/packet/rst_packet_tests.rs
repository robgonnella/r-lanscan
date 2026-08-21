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
    let sequence_number: u32 = 1;
    let rst_packet = RstPacketBuilder::default()
        .source_ip(source_ip)
        .source_mac(source_mac)
        .source_port(source_port)
        .dest_ip(target_ip)
        .dest_mac(target_mac)
        .dest_port(target_port)
        .sequence_number(sequence_number)
        .build()
        .unwrap();
    let packet = rst_packet.to_raw();
    assert!(!packet.is_empty());
}
