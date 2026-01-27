//! Provides helpers for creating SYN packets

use std::net;

use derive_builder::Builder;
use pnet::{
    packet::{MutablePacket, ethernet, ip, ipv4, tcp},
    util,
};

const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_IP4_SIZE: usize = ipv4::Ipv4Packet::minimum_packet_size();
const PKT_TCP_SIZE: usize = tcp::TcpPacket::minimum_packet_size();
const PKT_TOTAL_SIZE: usize = PKT_ETH_SIZE + PKT_IP4_SIZE + PKT_TCP_SIZE;

/// Represents a generator for raw SYN packets
#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct SynPacket {
    /// IP address of the host machine performing scanning
    source_ip: net::Ipv4Addr,
    /// MAC address of host machine performing scanning
    source_mac: util::MacAddr,
    /// Source port on which host machine is listening for packets
    source_port: u16,
    /// Target destination IP for the packet
    dest_ip: net::Ipv4Addr,
    /// Target destination MAC address for the packet
    dest_mac: util::MacAddr,
    /// Target destination port for the packet
    dest_port: u16,
}

impl SynPacket {
    /// Builds a new SYN request packet using the provided information
    pub fn to_raw(&self) -> [u8; PKT_TOTAL_SIZE] {
        let mut pkt_buf = [0u8; PKT_TOTAL_SIZE];

        let mut eth_header = ethernet::MutableEthernetPacket::new(&mut pkt_buf)
            .expect("failed to generate ethernet header");
        eth_header.set_ethertype(ethernet::EtherTypes::Ipv4);
        eth_header.set_source(self.source_mac);
        eth_header.set_destination(self.dest_mac);

        // set ip header
        let mut ip_buffer = [0u8; PKT_IP4_SIZE + PKT_TCP_SIZE];

        let mut ip_header = ipv4::MutableIpv4Packet::new(&mut ip_buffer)
            .expect("failed to generate ip header");

        ip_header.set_next_level_protocol(ip::IpNextHeaderProtocols::Tcp);
        ip_header.set_source(self.source_ip);
        ip_header.set_destination(self.dest_ip);
        ip_header.set_version(4);
        ip_header.set_ttl(64);
        ip_header.set_identification(0);
        ip_header.set_header_length(5);
        ip_header.set_total_length(40);
        ip_header.set_checksum(ipv4::checksum(&ip_header.to_immutable()));

        // set tcp header
        let mut tcp_buffer = [0u8; PKT_TCP_SIZE];

        let mut tcp_header = tcp::MutableTcpPacket::new(&mut tcp_buffer)
            .expect("failed to generate tcp header");

        tcp_header.set_source(self.source_port);
        tcp_header.set_destination(self.dest_port);
        tcp_header.set_flags(tcp::TcpFlags::SYN);
        tcp_header.set_data_offset(5);
        tcp_header.set_sequence(0);
        tcp_header.set_checksum(tcp::ipv4_checksum(
            &tcp_header.to_immutable(),
            &self.source_ip,
            &self.dest_ip,
        ));

        ip_header.set_payload(tcp_header.packet_mut());
        eth_header.set_payload(ip_header.packet_mut());

        pkt_buf
    }
}

#[cfg(test)]
#[allow(warnings)]
#[doc(hidden)]
// only used in tests
pub fn create_syn_reply(
    from_mac: util::MacAddr,
    from_ip: net::Ipv4Addr,
    from_port: u16,
    to_mac: util::MacAddr,
    to_ip: net::Ipv4Addr,
    to_port: u16,
    packet: &'static mut [u8; PKT_TOTAL_SIZE],
) {
    let mut eth_header = ethernet::MutableEthernetPacket::new(packet)
        .expect("failed to generate ethernet header");
    eth_header.set_ethertype(ethernet::EtherTypes::Ipv4);
    eth_header.set_source(from_mac);
    eth_header.set_destination(to_mac);

    // set ip header
    let mut ip_buffer = [0u8; PKT_IP4_SIZE + PKT_TCP_SIZE];

    let mut ip_header = ipv4::MutableIpv4Packet::new(&mut ip_buffer)
        .expect("failed to generate ip header");

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

    let mut tcp_header = tcp::MutableTcpPacket::new(&mut tcp_buffer)
        .expect("failed to generate tcp header");

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
}

#[cfg(test)]
#[path = "./syn_packet_tests.rs"]
mod tests;
