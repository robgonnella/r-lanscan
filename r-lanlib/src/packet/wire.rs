use pnet::datalink;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

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

// Implements the Reader trait for our PNet implementation
impl Reader for PNetReader {
    fn next_packet(&mut self) -> Result<&[u8], std::io::Error> {
        self.receiver.next()
    }
}

unsafe impl Sync for PNetReader {}

/**
 * A PNetSender implementation of packet Sender
 */
pub struct PNetSender {
    sender: Box<dyn datalink::DataLinkSender>,
}

// Implements the Sender trait for our PNet implementation
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

unsafe impl Sync for PNetSender {}

pub fn default(
    interface: &NetworkInterface,
) -> Result<(Arc<Mutex<dyn Reader>>, Arc<Mutex<dyn Sender>>), Box<dyn Error>> {
    let cfg = pnet::datalink::Config::default();

    let channel = match pnet::datalink::channel(&interface.into(), cfg) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to create packet reader",
        )),
        Err(e) => Err(e),
    }?;

    Ok((
        Arc::new(Mutex::new(PNetReader {
            receiver: channel.1,
        })),
        Arc::new(Mutex::new(PNetSender { sender: channel.0 })),
    ))
}

#[cfg(test)]
mod tests {
    use mockall::mock;

    use crate::network;

    use super::*;

    const SINGLE_BYTE: [u8; 1] = [1];

    mock! {
        PacketReader {}
        impl Reader for PacketReader {
            fn next_packet(&mut self) -> Result<&'static [u8], std::io::Error>;
        }
    }

    #[test]
    fn creates_default_wire() {
        let interface = network::get_default_interface().unwrap();
        let wire = default(&interface);
        assert!(wire.is_ok())
    }

    #[test]
    fn returns_packet_result() {
        let mut mock = MockPacketReader::new();
        mock.expect_next_packet().return_once(|| Ok(&SINGLE_BYTE));
        let result = mock.next_packet();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &SINGLE_BYTE);
    }
}
