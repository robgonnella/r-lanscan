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

// Implements the Reader trait for our PNet implementation
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

unsafe impl Send for PNetSender {}
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
    use datalink::MacAddr;
    use std::{net::Ipv4Addr, str::FromStr};

    use super::*;

    fn get_default_interface() -> NetworkInterface {
        NetworkInterface {
            cidr: "172.17.0.1/24".to_string(),
            description: "description".to_string(),
            flags: 0,
            index: 1,
            ips: Vec::new(),
            ipv4: Ipv4Addr::from_str("172.17.0.1").unwrap(),
            mac: MacAddr::from_str("00:00:00:00:00:00").unwrap(),
            name: "en0".to_string(),
        }
    }

    #[test]
    fn creates_default_wire() {
        let interface = get_default_interface();
        let wire = default(&interface);
        assert!(wire.is_ok())
    }
}
