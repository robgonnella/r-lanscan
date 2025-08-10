use std::str::FromStr;

use util::MacAddr;

use super::*;

#[test]
fn creates_arp_packet() {
    let source_ip = net::Ipv4Addr::from_str("192.168.68.1").unwrap();
    let source_mac = MacAddr::from_str("00:00:00:00:00:00").unwrap();
    let target_ip = net::Ipv4Addr::from_str("192.168.68.2").unwrap();
    let packet = ARPPacket::new(source_ip, source_mac, target_ip);
    assert!(!packet.is_empty());
}
