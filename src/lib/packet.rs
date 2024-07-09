pub mod pcap_reader;

/**
 * Although we're creating a generic trait for capturing packets
 * we still use the pcap::Packet and pcap::Error as these are probably
 * generic enough while still allowing for custom implementations
 *
 * TODO: look into returning custom types for Packet and Error
 */
pub trait Reader {
    fn next_packet(&mut self) -> Result<pcap::Packet, pcap::Error>;
}
