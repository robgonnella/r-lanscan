use std::sync::Arc;

use pnet::datalink::{self, DataLinkReceiver, DataLinkSender, NetworkInterface};

use crate::packet::{Reader, Sender};

/**
 * A BPF implementation of packet Reader
 */
pub struct BPFReader {
    receiver: Box<dyn datalink::DataLinkReceiver>,
}

// This bit of magic is required for BPF to be thread safe
// TODO: learn more about how this works
unsafe impl Send for BPFReader {}
unsafe impl Sync for BPFReader {}

// Implements the PacketReader trait for our BPF implementation
impl Reader for BPFReader {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error> {
        self.receiver.next()
    }
}

/**
 * A BPF implementation of packet Sender
 */
pub struct BPFSender {
    sender: Box<dyn datalink::DataLinkSender>,
}

// This bit of magic is required for BPF to be thread safe
// TODO: learn more about how this works
unsafe impl Send for BPFSender {}
unsafe impl Sync for BPFSender {}

impl Sender for BPFSender {
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
    let cfg = pnet::datalink::bpf::Config::default();

    let channel: (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
        match pnet::datalink::bpf::channel(Arc::clone(&interface).as_ref(), cfg) {
            Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Channel error: {e}"),
        };

    Box::new(BPFReader {
        receiver: channel.1,
    })
}

pub fn new_sender(interface: Arc<NetworkInterface>) -> Box<dyn Sender> {
    let cfg = pnet::datalink::bpf::Config::default();

    let channel: (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
        match pnet::datalink::bpf::channel(Arc::clone(&interface).as_ref(), cfg) {
            Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Channel error: {e}"),
        };

    Box::new(BPFSender { sender: channel.0 })
}
