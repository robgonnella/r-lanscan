use std::sync::Arc;

use pnet::datalink;

pub mod bpf;
pub mod pcap;

pub trait Reader: Send + Sync {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error>;
}

pub trait Sender: Send + Sync {
    fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error>;
}

pub type PacketReaderFactory = fn(Arc<datalink::NetworkInterface>) -> Box<dyn Reader>;
pub type PacketSenderFactory = fn(Arc<datalink::NetworkInterface>) -> Box<dyn Sender>;
