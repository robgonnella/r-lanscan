use std::net;

use pnet::{
    packet::{ethernet, ip, ipv4, tcp, MutablePacket},
    util,
};

const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_IP4_SIZE: usize = ipv4::Ipv4Packet::minimum_packet_size();
const PKT_TCP_SIZE: usize = tcp::TcpPacket::minimum_packet_size();
const PKT_TOTAL_SIZE: usize = PKT_ETH_SIZE + PKT_IP4_SIZE + PKT_TCP_SIZE;

pub struct SYNPacket {}

impl SYNPacket {
    pub fn new(
        source_mac: util::MacAddr,
        source_ipv4: net::Ipv4Addr,
        source_port: u16,
        dest_ipv4: net::Ipv4Addr,
        dest_mac: util::MacAddr,
        dest_port: u16,
    ) -> [u8; PKT_TOTAL_SIZE] {
        let mut pkt_buf = [0u8; PKT_TOTAL_SIZE];

        let mut eth_header = ethernet::MutableEthernetPacket::new(&mut pkt_buf)
            .expect("failed to generate ethernet header");
        eth_header.set_ethertype(ethernet::EtherTypes::Ipv4);
        eth_header.set_source(source_mac);
        eth_header.set_destination(dest_mac);

        // set ip header
        let mut ip_buffer = [0u8; PKT_IP4_SIZE + PKT_TCP_SIZE];

        let mut ip_header =
            ipv4::MutableIpv4Packet::new(&mut ip_buffer).expect("failed to generate ip header");

        ip_header.set_next_level_protocol(ip::IpNextHeaderProtocols::Tcp);
        ip_header.set_source(source_ipv4);
        ip_header.set_destination(dest_ipv4);
        ip_header.set_version(4);
        ip_header.set_ttl(64);
        ip_header.set_identification(0);
        ip_header.set_header_length(5);
        ip_header.set_total_length(40);
        ip_header.set_checksum(ipv4::checksum(&ip_header.to_immutable()));

        // set tcp header
        let mut tcp_buffer = [0u8; PKT_TCP_SIZE];

        let mut tcp_header =
            tcp::MutableTcpPacket::new(&mut tcp_buffer).expect("failed to generate tcp header");

        tcp_header.set_source(source_port);
        tcp_header.set_destination(dest_port);
        tcp_header.set_flags(tcp::TcpFlags::SYN);
        tcp_header.set_data_offset(5);
        tcp_header.set_sequence(0);
        tcp_header.set_checksum(tcp::ipv4_checksum(
            &tcp_header.to_immutable(),
            &source_ipv4,
            &dest_ipv4,
        ));

        ip_header.set_payload(tcp_header.packet_mut());
        eth_header.set_payload(ip_header.packet_mut());

        pkt_buf
    }
}

#[cfg(test)]
#[allow(warnings)]
pub fn create_syn_reply(
    from_mac: util::MacAddr,
    from_ip: net::Ipv4Addr,
    from_port: u16,
    to_mac: util::MacAddr,
    to_ip: net::Ipv4Addr,
    to_port: u16,
) -> &'static [u8; PKT_TOTAL_SIZE] {
    static mut PACKET: [u8; PKT_TOTAL_SIZE] = [0u8; PKT_TOTAL_SIZE];

    let mut eth_header = ethernet::MutableEthernetPacket::new(unsafe { &mut PACKET })
        .expect("failed to generate ethernet header");
    eth_header.set_ethertype(ethernet::EtherTypes::Ipv4);
    eth_header.set_source(from_mac);
    eth_header.set_destination(to_mac);

    // set ip header
    let mut ip_buffer = [0u8; PKT_IP4_SIZE + PKT_TCP_SIZE];

    let mut ip_header =
        ipv4::MutableIpv4Packet::new(&mut ip_buffer).expect("failed to generate ip header");

    ip_header.set_next_level_protocol(ip::IpNextHeaderProtocols::Tcp);
    ip_header.set_source(from_ip);
    ip_header.set_destination(to_ip);
    ip_header.set_version(4);
    ip_header.set_ttl(64);
    ip_header.set_identification(0);
    ip_header.set_header_length(5);
    ip_header.set_total_length(40);
    ip_header.set_checksum(ipv4::checksum(&ip_header.to_immutable()));

    // set tcp header
    let mut tcp_buffer = [0u8; PKT_TCP_SIZE];

    let mut tcp_header =
        tcp::MutableTcpPacket::new(&mut tcp_buffer).expect("failed to generate tcp header");

    tcp_header.set_source(from_port);
    tcp_header.set_destination(to_port);
    tcp_header.set_flags(tcp::TcpFlags::SYN | tcp::TcpFlags::ACK);
    tcp_header.set_data_offset(5);
    tcp_header.set_sequence(11111);
    tcp_header.set_checksum(tcp::ipv4_checksum(
        &tcp_header.to_immutable(),
        &from_ip,
        &to_ip,
    ));

    ip_header.set_payload(tcp_header.packet_mut());
    eth_header.set_payload(ip_header.packet_mut());

    unsafe { &PACKET }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use util::MacAddr;

    use super::*;

    #[test]
    fn creates_syn_packet() {
        let source_ip = net::Ipv4Addr::from_str("192.168.68.1").unwrap();
        let source_mac = MacAddr::from_str("00:00:00:00:00:00").unwrap();
        let source_port: u16 = 54321;
        let target_ip = net::Ipv4Addr::from_str("192.168.68.2").unwrap();
        let target_mac = MacAddr::from_str("00:00:00:00:00:01").unwrap();
        let target_port: u16 = 22;
        let packet = SYNPacket::new(
            source_mac,
            source_ip,
            source_port,
            target_ip,
            target_mac,
            target_port,
        );
        assert!(!packet.is_empty());
    }
}
