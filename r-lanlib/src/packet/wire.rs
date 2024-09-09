use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use pnet::datalink;

use crate::{
    network::NetworkInterface,
    packet::{Reader, Sender},
};

/**
 * A PNetReader implementation of packet Reader
 */
pub struct PNetReader {
    receiver: Box<dyn datalink::DataLinkReceiver>,
}

// Implements the PacketReader trait for our BPF implementation
impl Reader for PNetReader {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error> {
        self.receiver.next()
    }
}

unsafe impl Send for PNetReader {}
unsafe impl Sync for PNetReader {}

/**
 * A PNetSender implementation of packet Sender
 */
pub struct PNetSender {
    sender: Box<dyn datalink::DataLinkSender>,
}

impl Sender for PNetSender {
    fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error> {
        let opt = self.sender.send_to(packet, None);
        match opt {
            Some(res) => res,
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to send packet",
            )),
        }
    }
}

unsafe impl Send for PNetSender {}
unsafe impl Sync for PNetSender {}

pub fn new_default_reader(
    interface: &NetworkInterface,
) -> Result<Arc<Mutex<dyn Reader>>, Box<dyn Error>> {
    let cfg = pnet::datalink::Config::default();

    let channel = match pnet::datalink::channel(&interface.into(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to create packet reader",
        )),
        Err(e) => Err(e),
    }?;

    Ok(Arc::new(Mutex::new(PNetReader {
        receiver: channel.1,
    })))
}

pub fn new_default_sender(
    interface: &NetworkInterface,
) -> Result<Arc<Mutex<dyn Sender>>, Box<dyn Error>> {
    let cfg = pnet::datalink::Config::default();

    let channel = match pnet::datalink::channel(&interface.into(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to create packet sender",
        )),
        Err(e) => Err(e),
    }?;

    Ok(Arc::new(Mutex::new(PNetSender { sender: channel.0 })))
}
