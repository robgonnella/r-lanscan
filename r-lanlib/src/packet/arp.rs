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

pub struct ARPPacket {}

impl ARPPacket {
    pub fn new(
        source_ipv4: net::Ipv4Addr,
        source_mac: util::MacAddr,
        target_ipv4: net::Ipv4Addr,
    ) -> [u8; PKT_TOTAL_SIZE] {
        let mut pkt_buf = [0u8; PKT_TOTAL_SIZE];

        // Use scope blocks so we can re-borrow our buffer
        {
            // Build our base ethernet frame
            let mut pkt_eth = ethernet::MutableEthernetPacket::new(&mut pkt_buf)
                .expect("failed to generate ethernet packet");

            pkt_eth.set_destination(util::MacAddr::broadcast());
            pkt_eth.set_source(source_mac);
            pkt_eth.set_ethertype(ethernet::EtherTypes::Arp);
        }

        {
            // Build the ARP frame on top of the ethernet frame
            let mut pkt_arp = arp::MutableArpPacket::new(&mut pkt_buf[PKT_ARP_OFFSET..])
                .expect("failed to generate arp packet");

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
}

#[cfg(test)]
mod tests {

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
}
