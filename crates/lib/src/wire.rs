//! Implements a default Wire using pnet

use pnet::datalink::{self, PacketMetadata as PnetPacketMetadata};
use std::{
    sync::{Arc, Mutex},
    time,
};

use crate::{
    error::{RLanLibError, Result},
    network::NetworkInterface,
};

/// Default timing for throttling packet sends to prevent packet loss.
/// 200µs (5,000 pps) balances scan speed against reliability on WiFi,
/// macOS BPF, and virtualised environments where tighter timings cause
/// silent packet drops.
pub const DEFAULT_PACKET_SEND_TIMING: time::Duration =
    time::Duration::from_micros(200);

/// PacketMetadata from wire
pub type PacketMetadata = PnetPacketMetadata;

/// Trait describing a packet reader
pub trait Reader: Send {
    /// Returns the next packet off of the wire
    fn next_packet(&mut self) -> Result<&[u8]>;
    /// Returns the next packet off of the wire along with metadata
    fn next_packet_with_metadata(&mut self) -> Result<(&[u8], PacketMetadata)>;
}

/// Trait describing a packet sender
pub trait Sender: Send {
    /// Should send a packet over the wire
    fn send(&mut self, packet: &[u8]) -> Result<()>;
}

/// Represents a packet Reader and packet Sender tuple
#[derive(Clone)]
pub struct Wire(pub Arc<Mutex<dyn Sender>>, pub Arc<Mutex<dyn Reader>>);

/// A PNetReader implementation of packet Reader
pub struct PNetReader {
    receiver: Box<dyn datalink::DataLinkReceiver>,
}

// Implements the Reader trait for our PNet implementation
impl Reader for PNetReader {
    fn next_packet(&mut self) -> Result<&[u8]> {
        self.receiver
            .next()
            .map_err(|e| RLanLibError::Wire(e.to_string()))
    }

    fn next_packet_with_metadata(&mut self) -> Result<(&[u8], PacketMetadata)> {
        self.receiver
            .next_with_metadata()
            .map_err(|e| RLanLibError::Wire(e.to_string()))
    }
}

/// A PNetSender implementation of packet Sender
pub struct PNetSender {
    sender: Box<dyn datalink::DataLinkSender>,
}

// Implements the Sender trait for our PNet implementation
impl Sender for PNetSender {
    fn send(&mut self, packet: &[u8]) -> Result<()> {
        let opt = self.sender.send_to(packet, None);
        match opt {
            Some(res) => {
                Ok(res.map_err(|e| RLanLibError::Wire(e.to_string()))?)
            }
            None => Err(RLanLibError::Wire("failed to send packet".into())),
        }
    }
}

/// Returns the default wire for current host
///
/// Example
/// ```no_run
/// # use std::io;
/// # use r_lanlib::network;
/// # use r_lanlib::wire;
/// let interface = network::get_default_interface().unwrap();
/// let packet_wire = wire::default(&interface).unwrap();
/// ```
pub fn default(interface: &NetworkInterface) -> Result<Wire> {
    let cfg = pnet::datalink::Config {
        enable_timestamps: true,
        read_buffer_size: 65536, // 64 KB — holds ~43 max-size frames
        write_buffer_size: 65536, // 64 KB — consistent with raw socket convention
        ..pnet::datalink::Config::default()
    };

    let channel = match pnet::datalink::channel(&interface.into(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => {
            Err(RLanLibError::Wire("failed to create packet reader".into()))
        }
        Err(e) => Err(RLanLibError::Wire(e.to_string())),
    }?;

    Ok(Wire(
        Arc::new(Mutex::new(PNetSender { sender: channel.0 })),
        Arc::new(Mutex::new(PNetReader {
            receiver: channel.1,
        })),
    ))
}

#[cfg(test)]
#[path = "./wire_tests.rs"]
mod tests;

/// Provides wire mocks for other modules in test
#[cfg(test)]
pub mod mocks {
    use mockall::mock;

    use super::*;

    mock! {
            pub PacketReader {}
            impl Reader for PacketReader {
                fn next_packet(&mut self) -> Result<&'static [u8]>;
                fn next_packet_with_metadata(&mut self) -> Result<(&'static [u8], PacketMetadata)>;
            }
    }

    mock! {
        pub PacketSender {}
        impl Sender for PacketSender {
            fn send(&mut self, packet: &[u8]) -> Result<()>;
        }
    }
}
