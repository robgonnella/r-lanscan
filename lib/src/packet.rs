//! Provides helpers for creating and sending packets

use core::time;
use std::error::Error;

pub mod arp_packet;
mod heartbeat_packet;
pub mod rst_packet;
pub mod syn_packet;
pub mod wire;

pub use heartbeat_packet::build as build_heartbeat_packet;

/// Default timing for throttling packet sends to prevent packet loss
pub const DEFAULT_PACKET_SEND_TIMING: time::Duration = time::Duration::from_micros(50);

/// Trait describing a packet reader
pub trait Reader: Send + Sync {
    /// Should return the next packet off of the wire
    fn next_packet(&mut self) -> Result<&[u8], Box<dyn Error>>;
}

/// Trait describing a packet sender
pub trait Sender: Send + Sync {
    /// Should send a packet over the wire
    fn send(&mut self, packet: &[u8]) -> Result<(), Box<dyn Error>>;
}

#[cfg(test)]
#[path = "./packet_tests.rs"]
#[doc(hidden)]
pub mod mocks;
