use core::time;

use crate::network::NetworkInterface;

pub mod arp;
pub mod syn;
pub mod wire;

/// Default timing for throttling packet sends to prevent packet loss
pub const DEFAULT_PACKET_SEND_TIMING: time::Duration = time::Duration::from_micros(100);

pub trait Reader: Send + Sync {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error>;
}

pub trait Sender: Send + Sync {
    fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error>;
}

pub type PacketReaderFactory = fn(&NetworkInterface) -> Box<dyn Reader>;
pub type PacketSenderFactory = fn(&NetworkInterface) -> Box<dyn Sender>;
