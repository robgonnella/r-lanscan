use std::sync;

use pnet::datalink;

pub mod arp;
pub mod syn;
pub mod wire;

pub trait Reader: Send + Sync {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error>;
}

pub trait Sender: Send + Sync {
    fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error>;
}

pub type PacketReaderFactory = fn(sync::Arc<datalink::NetworkInterface>) -> Box<dyn Reader>;
pub type PacketSenderFactory = fn(sync::Arc<datalink::NetworkInterface>) -> Box<dyn Sender>;

pub const LISTEN_PORT: u16 = 54322;
