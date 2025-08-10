//! Implements a default Wire using pnet

use pnet::datalink;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use crate::{
    network::NetworkInterface,
    packet::{Reader, Sender},
};

/// Represents a packet Reader and packet Sender tuple
pub type Wire = (Arc<Mutex<dyn Reader>>, Arc<Mutex<dyn Sender>>);

/// A PNetReader implementation of packet Reader
pub struct PNetReader {
    receiver: Box<dyn datalink::DataLinkReceiver>,
}

// Implements the Reader trait for our PNet implementation
impl Reader for PNetReader {
    fn next_packet(&mut self) -> Result<&[u8], Box<dyn Error>> {
        self.receiver.next().map_err(Box::from)
    }
}

unsafe impl Sync for PNetReader {}

/// A PNetSender implementation of packet Sender
pub struct PNetSender {
    sender: Box<dyn datalink::DataLinkSender>,
}

// Implements the Sender trait for our PNet implementation
impl Sender for PNetSender {
    fn send(&mut self, packet: &[u8]) -> Result<(), Box<dyn Error>> {
        let opt = self.sender.send_to(packet, None);
        match opt {
            Some(res) => Ok(res?),
            None => Err(Box::from("failed to send packet")),
        }
    }
}

unsafe impl Sync for PNetSender {}

/// Returns the default wire for current host
///
/// Example
/// ```no_run
/// # use std::io;
/// # use r_lanlib::network;
/// # use r_lanlib::packet::wire;
/// let interface = network::get_default_interface().unwrap();
/// let packet_wire = wire::default(&interface).unwrap();
/// ```
pub fn default(interface: &NetworkInterface) -> Result<Wire, Box<dyn Error>> {
    let cfg = pnet::datalink::Config::default();

    let channel = match pnet::datalink::channel(&interface.into(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => {
            let e: Box<dyn Error> = Box::from("failed to create packet reader");
            Err(e)
        }
        Err(e) => Err(Box::from(e.to_string())),
    }?;

    Ok((
        Arc::new(Mutex::new(PNetReader {
            receiver: channel.1,
        })),
        Arc::new(Mutex::new(PNetSender { sender: channel.0 })),
    ))
}

#[cfg(test)]
#[path = "./wire_tests.rs"]
mod tests;
