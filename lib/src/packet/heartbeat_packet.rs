//! Provides helpers for creating heartbeat packets

use std::net;

use pnet::{
    packet::{MutablePacket, ethernet, ip, ipv4, tcp},
    util,
};

const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_IP4_SIZE: usize = ipv4::Ipv4Packet::minimum_packet_size();
const PKT_TCP_SIZE: usize = tcp::TcpPacket::minimum_packet_size();
const PKT_TOTAL_SIZE: usize = PKT_ETH_SIZE + PKT_IP4_SIZE + PKT_TCP_SIZE;

/// Builds a new heartbeat packet targeting the provided source
/// This is an arbitrary bit of information sent on an interval as a TCP
/// packet as a heartbeat indicator
pub fn build(
    source_mac: util::MacAddr,
    source_ipv4: net::Ipv4Addr,
    source_port: u16,
) -> [u8; PKT_TOTAL_SIZE] {
    let mut pkt_buf = [0u8; PKT_TOTAL_SIZE];

    let mut eth_header = ethernet::MutableEthernetPacket::new(&mut pkt_buf)
        .expect("failed to generate ethernet header");
    eth_header.set_ethertype(ethernet::EtherTypes::Ipv4);
    eth_header.set_source(source_mac);
    eth_header.set_destination(source_mac);

    // set ip header
    let mut ip_buffer = [0u8; PKT_IP4_SIZE + PKT_TCP_SIZE];

    let mut ip_header =
        ipv4::MutableIpv4Packet::new(&mut ip_buffer).expect("failed to generate ip header");

    ip_header.set_next_level_protocol(ip::IpNextHeaderProtocols::Tcp);
    ip_header.set_source(source_ipv4);
    ip_header.set_destination(source_ipv4);
    ip_header.set_version(4);
    ip_header.set_ttl(64);
    ip_header.set_identification(0);
    ip_header.set_header_length(5);
    ip_header.set_total_length(40);
    // ip_header.set_flags(ipv4::Ipv4Flags::DontFragment);
    ip_header.set_checksum(ipv4::checksum(&ip_header.to_immutable()));

    // set tcp header
    let mut tcp_buffer = [0u8; PKT_TCP_SIZE];

    let mut tcp_header =
        tcp::MutableTcpPacket::new(&mut tcp_buffer).expect("failed to generate tcp header");

    tcp_header.set_source(source_port);
    tcp_header.set_destination(source_port);
    tcp_header.set_flags(tcp::TcpFlags::SYN);
    tcp_header.set_data_offset(5);
    tcp_header.set_sequence(0);
    tcp_header.set_checksum(tcp::ipv4_checksum(
        &tcp_header.to_immutable(),
        &source_ipv4,
        &source_ipv4,
    ));

    ip_header.set_payload(tcp_header.packet_mut());
    eth_header.set_payload(ip_header.packet_mut());

    pkt_buf
}

#[cfg(test)]
#[path = "./heartbeat_packet_tests.rs"]
mod tests;
