use std::sync::Arc;

use pnet::datalink::NetworkInterface;

use super::{Reader, Sender};

/**
 * A PCAP implementation of packet Reader
 */
pub struct PCapReader {
    cap: pcap::Capture<pcap::Active>,
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
        let res = self.cap.next_packet();
        match res {
            Ok(packet) => Ok(packet.data),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )),
        }
    }
}

/**
 * A PCAP implementation of packet Sender
 */
pub struct PCapSender {
    cap: pcap::Capture<pcap::Active>,
}

// This bit of magic is required for PCap to be thread safe
// TODO: learn more about how this works
unsafe impl Send for PCapSender {}
unsafe impl Sync for PCapSender {}

impl Sender for PCapSender {
    fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error> {
        let res = self.cap.sendpacket(packet);
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )),
        }
    }
}

/**
 * The important bit here is Arc<Mutex<Box<dyn Reader + Send + Sync>>>
 * There's a lot going on in this type
 *
 * - Arc -> Allow this data to be shared by multiple owners in a thread safe way
 *          as opposed to Rc which shares data with multiple owners in a non-
 *          thread safe way
 * - Mutex -> Allow the internal data to be accessed in a mutable way while
 *            the container remains immutable in a thread safe way, as opposed
 *            to the non-thread-safe alternative "RefCell". Essentially the
 *            internal structure, Capture<Active> for pcap, needs to be mutable
 *            to read packets, but our encapsulation Reader should still
 *            be immutable. Mutex allows this to be possible in a thread safe
 *            way.
 * - Box -> Because we allow the user to implement their own packet capturer
 *          if they choose, we use a trait (Reader), but this means the
 *          compiler can't know the size at compile time. To deal with this
 *          we allocate space on the Heap using Box and pass around the
 *          reference which DOES have a known size at compile time.
 * + Send + Syn -> Indicates that Reader is thread safe and can be safely
 *                 synchronized across threads.
 */
pub fn new_reader(interface: Arc<NetworkInterface>) -> Box<dyn Reader> {
    let cap = pcap::Capture::from_device(interface.name.as_ref())
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");

    Box::new(PCapReader { cap })
}

pub fn new_sender(interface: Arc<NetworkInterface>) -> Box<dyn Sender> {
    let cap = pcap::Capture::from_device(interface.name.as_ref())
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");

    Box::new(PCapSender { cap })
}
