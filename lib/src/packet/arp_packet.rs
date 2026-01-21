//! Provides helpers for creating ARP packets

use derive_builder::Builder;
use pnet::{
    packet::{MutablePacket, arp, ethernet},
    util,
};
use std::net;

// Constants used to help locate our nested packets
const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = arp::ArpPacket::minimum_packet_size();
const PKT_TOTAL_SIZE: usize = PKT_ETH_SIZE + PKT_ARP_SIZE;

/// Represents a generator for raw ARP packets
#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct ArpPacket {
    /// Ip of the host sending the packet
    source_ip: net::Ipv4Addr,
    /// Mac address of the host sending the packet
    source_mac: util::MacAddr,
    /// Target destination IP for the packet
    dest_ip: net::Ipv4Addr,
}

impl ArpPacket {
    /// Builds a new ARP request packet based on provided information
    /// This is what the internals of the arp_scanner will send when scanning
    /// for devices on the network
    pub fn to_raw(&self) -> [u8; PKT_TOTAL_SIZE] {
        let mut pkt_buf = [0u8; PKT_TOTAL_SIZE];

        // Build our base ethernet frame
        let mut pkt_eth = ethernet::MutableEthernetPacket::new(&mut pkt_buf)
            .expect("failed to generate ethernet packet");

        let mut arp_buffer = [0u8; PKT_ARP_SIZE];

        let mut pkt_arp =
            arp::MutableArpPacket::new(&mut arp_buffer).expect("failed to generate arp packet");

        pkt_eth.set_destination(util::MacAddr::broadcast());
        pkt_eth.set_source(self.source_mac);
        pkt_eth.set_ethertype(ethernet::EtherTypes::Arp);

        pkt_arp.set_hardware_type(arp::ArpHardwareTypes::Ethernet);
        pkt_arp.set_protocol_type(ethernet::EtherTypes::Ipv4);
        pkt_arp.set_hw_addr_len(6);
        pkt_arp.set_proto_addr_len(4);
        pkt_arp.set_operation(arp::ArpOperations::Request);
        pkt_arp.set_sender_hw_addr(self.source_mac);
        pkt_arp.set_sender_proto_addr(self.source_ip);
        pkt_arp.set_target_hw_addr(util::MacAddr::zero());
        pkt_arp.set_target_proto_addr(self.dest_ip);

        pkt_eth.set_payload(pkt_arp.packet_mut());

        pkt_buf
    }
}

#[cfg(test)]
#[allow(warnings)]
#[doc(hidden)]
// only used in tests
pub fn create_arp_reply(
    from_mac: util::MacAddr,
    from_ip: net::Ipv4Addr,
    to_mac: util::MacAddr,
    to_ip: net::Ipv4Addr,
    packet: &'static mut [u8; PKT_TOTAL_SIZE],
) {
    let mut pkt_eth =
        ethernet::MutableEthernetPacket::new(packet).expect("failed to generate ethernet packet");

    let mut arp_buffer = [0u8; PKT_ARP_SIZE];

    let mut pkt_arp =
        arp::MutableArpPacket::new(&mut arp_buffer).expect("failed to generate arp packet");

    pkt_eth.set_destination(to_mac);
    pkt_eth.set_source(from_mac);
    pkt_eth.set_ethertype(ethernet::EtherTypes::Arp);

    pkt_arp.set_hardware_type(arp::ArpHardwareTypes::Ethernet);
    pkt_arp.set_protocol_type(ethernet::EtherTypes::Ipv4);
    pkt_arp.set_hw_addr_len(6);
    pkt_arp.set_proto_addr_len(4);
    pkt_arp.set_operation(arp::ArpOperations::Reply);
    pkt_arp.set_sender_hw_addr(from_mac);
    pkt_arp.set_sender_proto_addr(from_ip);
    pkt_arp.set_target_hw_addr(to_mac);
    pkt_arp.set_target_proto_addr(to_ip);

    pkt_eth.set_payload(pkt_arp.packet_mut());
}

#[cfg(test)]
#[path = "./arp_packet_tests.rs"]
mod tests;
