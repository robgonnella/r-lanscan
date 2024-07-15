use std::sync::{Arc, Mutex};

use pnet::datalink::{self, DataLinkReceiver, DataLinkSender, NetworkInterface};

use super::{Reader, Sender};

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
            Some(res) => Ok(()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to send packet",
            )),
        }
    }
}

/**
 * The important bit here is Arc<Mutex<Box<dyn PacketReader + Send + Sync>>>
 * There's a lot going on in this type
 *
 * - Arc -> Allow this data to be shared by multiple owners in a thread safe way
 *          as opposed to Rc which shares data with multiple owners in a non-
 *          thread safe way
 * - Mutex -> Allow the internal data to be accessed in a mutable way while
 *            the container remains immutable in a thread safe way, as opposed
 *            to the non-thread-safe alternative "RefCell". Essentially the
 *            internal structure, Capture<Active> for pcap, needs to be mutable
 *            to read packets, but our encapsulation PacketReader should still
 *            be immutable. Mutex allows this to be possible in a thread safe
 *            way.
 * - Box -> Because we allow the user to implement their own packet capturer
 *          if they choose, we use a trait (PacketReader), but this means the
 *          compiler can't know the size at compile time. To deal with this
 *          we allocate space on the Heap using Box and pass around the
 *          reference which DOES have a known size at compile time.
 * + Send + Syn -> Indicates that PacketReader is thread safe and can be safely
 *                 synchronized across threads.
 */
pub fn new_reader(interface: Arc<NetworkInterface>) -> Arc<Mutex<Box<dyn Reader>>> {
    let cfg = pnet::datalink::bpf::Config::default();

    let channel: (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
        match pnet::datalink::bpf::channel(Arc::clone(&interface).as_ref(), cfg) {
            Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Channel error: {e}"),
        };

    let boxed: Box<dyn Reader> = Box::new(BPFReader {
        receiver: channel.1,
    });
    Arc::new(Mutex::new(boxed))
}

pub fn new_sender(interface: Arc<NetworkInterface>) -> Arc<Mutex<Box<dyn Sender>>> {
    let cfg = pnet::datalink::bpf::Config::default();

    let channel: (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
        match pnet::datalink::bpf::channel(Arc::clone(&interface).as_ref(), cfg) {
            Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Channel error: {e}"),
        };

    let boxed: Box<dyn Sender> = Box::new(BPFSender { sender: channel.0 });
    Arc::new(Mutex::new(boxed))
}
