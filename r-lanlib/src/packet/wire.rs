use pnet::datalink::{self, DataLinkReceiver, DataLinkSender};

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
            Some(_res) => Ok(()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to send packet",
            )),
        }
    }
}

unsafe impl Send for PNetSender {}
unsafe impl Sync for PNetSender {}

pub fn new_default_reader(interface: &NetworkInterface) -> Box<dyn Reader> {
    let cfg = pnet::datalink::Config::default();

    let channel: (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
        match pnet::datalink::channel(&interface.into(), cfg) {
            Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Channel error: {e}"),
        };

    Box::new(PNetReader {
        receiver: channel.1,
    })
}

pub fn new_default_sender(interface: &NetworkInterface) -> Box<dyn Sender> {
    let cfg = pnet::datalink::Config::default();

    let channel: (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
        match pnet::datalink::channel(&interface.into(), cfg) {
            Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Channel error: {e}"),
        };

    Box::new(PNetSender { sender: channel.0 })
}
