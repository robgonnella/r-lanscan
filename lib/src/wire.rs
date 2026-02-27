//! Implements a default Wire using pnet

use pnet::datalink;
use std::sync::{Arc, Mutex};

use crate::{
    error::{RLanLibError, Result},
    network::NetworkInterface,
    packet::{Reader, Sender},
};

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
    let cfg = pnet::datalink::Config::default();

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
