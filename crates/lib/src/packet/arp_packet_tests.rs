use util::MacAddr;

use super::*;

#[test]
fn creates_arp_packet() {
    let source_ip = net::Ipv4Addr::new(192, 168, 68, 1);
    let source_mac = MacAddr::default();
    let target_ip = net::Ipv4Addr::new(192, 168, 68, 2);
    let arp_packet = ArpPacketBuilder::default()
        .source_ip(source_ip)
        .source_mac(source_mac)
        .dest_ip(target_ip)
        .build()
        .unwrap();
    let packet = arp_packet.to_raw();
    assert!(!packet.is_empty());
}
