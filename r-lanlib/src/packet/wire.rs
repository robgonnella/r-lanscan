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
    fn next_packet(&mut self) -> Result<&[u8], Box<dyn Error>> {
        self.receiver.next().or_else(|e| Err(Box::from(e)))
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
    fn send(&mut self, packet: &[u8]) -> Result<(), Box<dyn Error>> {
        let opt = self.sender.send_to(packet, None);
        match opt {
            Some(res) => Ok(res?),
            None => Err(Box::from("failed to send packet")),
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
mod tests {
    use crate::{
        network,
        packet::mocks::{MockPacketReader, MockPacketSender},
    };

    use super::*;

    #[test]
    fn creates_default_wire() {
        let interface = network::get_default_interface().unwrap();
        let wire = default(&interface);
        assert!(wire.is_ok());
    }

    #[test]
    fn returns_packet_result() {
        let mut mock = MockPacketReader::new();
        mock.expect_next_packet().returning(|| Ok(&[1]));
        let result = mock.next_packet();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &[1]);
    }

    #[test]
    fn send_packet() {
        let mut mock = MockPacketSender::new();
        mock.expect_send()
            .withf(|p| *p == [1])
            .returning(|_| Ok(()));
        let result = mock.send(&[1]);
        assert!(result.is_ok())
    }
}
