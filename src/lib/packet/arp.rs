use std::net;

use pnet::{
    packet::{arp, ethernet},
    util,
};

// Constants used to help locate our nested packets
const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = arp::ArpPacket::minimum_packet_size();
const PKT_ARP_OFFSET: usize = PKT_ETH_SIZE;
const PKT_TOTAL_SIZE: usize = PKT_ETH_SIZE + PKT_ARP_SIZE;

pub fn new(
    source_ipv4: net::Ipv4Addr,
    source_mac: util::MacAddr,
    target_ipv4: net::Ipv4Addr,
) -> [u8; PKT_TOTAL_SIZE] {
    let mut pkt_buf = [0u8; PKT_TOTAL_SIZE];

    // Use scope blocks so we can re-borrow our buffer
    {
        // Build our base ethernet frame
        let mut pkt_eth = ethernet::MutableEthernetPacket::new(&mut pkt_buf).unwrap();

        pkt_eth.set_destination(util::MacAddr::broadcast());
        pkt_eth.set_source(source_mac);
        pkt_eth.set_ethertype(ethernet::EtherTypes::Arp);
    }

    {
        // Build the ARP frame on top of the ethernet frame
        let mut pkt_arp = arp::MutableArpPacket::new(&mut pkt_buf[PKT_ARP_OFFSET..]).unwrap();

        pkt_arp.set_hardware_type(arp::ArpHardwareTypes::Ethernet);
        pkt_arp.set_protocol_type(ethernet::EtherTypes::Ipv4);
        pkt_arp.set_hw_addr_len(6);
        pkt_arp.set_proto_addr_len(4);
        pkt_arp.set_operation(arp::ArpOperations::Request);
        pkt_arp.set_sender_hw_addr(source_mac);
        pkt_arp.set_sender_proto_addr(source_ipv4);
        pkt_arp.set_target_hw_addr(util::MacAddr::zero());
        pkt_arp.set_target_proto_addr(target_ipv4);
    }

    pkt_buf
}
