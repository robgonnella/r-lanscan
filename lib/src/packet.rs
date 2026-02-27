//! Provides helpers for creating and sending packets

use core::time;

pub mod arp_packet;
pub mod heartbeat_packet;
pub mod rst_packet;
pub mod syn_packet;

use crate::error::Result;

/// Default timing for throttling packet sends to prevent packet loss.
/// 200µs (5,000 pps) balances scan speed against reliability on WiFi,
/// macOS BPF, and virtualised environments where tighter timings cause
/// silent packet drops.
pub const DEFAULT_PACKET_SEND_TIMING: time::Duration =
    time::Duration::from_micros(200);

/// Trait describing a packet reader
pub trait Reader: Send {
    /// Should return the next packet off of the wire
    fn next_packet(&mut self) -> Result<&[u8]>;
}

/// Trait describing a packet sender
pub trait Sender: Send {
    /// Should send a packet over the wire
    fn send(&mut self, packet: &[u8]) -> Result<()>;
}

#[cfg(test)]
#[path = "./packet_tests.rs"]
#[doc(hidden)]
pub mod mocks;
