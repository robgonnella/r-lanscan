use std::sync::Arc;

use pnet::datalink::{self, NetworkInterface};

use super::{Reader, Sender};

/**
 * A PCAP implementation of packet Reader
 */
pub struct PCapReader {
    receiver: Box<dyn datalink::DataLinkReceiver>,
}

// This bit of magic is required for PCap to be thread safe
// TODO: learn more about how this works
unsafe impl Send for PCapReader {}
unsafe impl Sync for PCapReader {}

// Implements the Reader trait for our PCAP implementation
// TODO: figure out why pcap reader isn't working - doesn't see packets
// works fine when pcap sender is paired with bpf reader but not
// when pcap sender is paired with pcap reader
impl Reader for PCapReader {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error> {
        self.receiver.next()
    }
}

/**
 * A PCAP implementation of packet Sender
 */
pub struct PCapSender {
    sender: Box<dyn datalink::DataLinkSender>,
}

// This bit of magic is required for PCap to be thread safe
// TODO: learn more about how this works
unsafe impl Send for PCapSender {}
unsafe impl Sync for PCapSender {}

impl Sender for PCapSender {
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

pub fn new_reader(interface: Arc<NetworkInterface>) -> Box<dyn Reader> {
    let cfg = pnet::datalink::Config::default();

    let channel: (
        Box<dyn datalink::DataLinkSender>,
        Box<dyn datalink::DataLinkReceiver>,
    ) = match pnet::datalink::channel(Arc::clone(&interface).as_ref(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Channel error: {e}"),
    };

    Box::new(PCapReader {
        receiver: channel.1,
    })
}

pub fn new_sender(interface: Arc<NetworkInterface>) -> Box<dyn Sender> {
    let cfg = pnet::datalink::Config::default();

    let channel: (
        Box<dyn datalink::DataLinkSender>,
        Box<dyn datalink::DataLinkReceiver>,
    ) = match pnet::datalink::channel(Arc::clone(&interface).as_ref(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Channel error: {e}"),
    };

    Box::new(PCapSender { sender: channel.0 })
}
