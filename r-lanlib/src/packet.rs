#[cfg(test)]
use mockall::{automock, predicate::*};

use core::time;

pub mod arp;
pub mod heartbeat;
pub mod syn;
pub mod wire;

/// Default timing for throttling packet sends to prevent packet loss
pub const DEFAULT_PACKET_SEND_TIMING: time::Duration = time::Duration::from_micros(50);

pub trait Reader: Send + Sync {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error>;
}

#[cfg_attr(test, automock)]
pub trait Sender: Send + Sync {
    fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error>;
}
