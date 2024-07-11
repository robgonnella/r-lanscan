use std::sync::{Arc, Mutex};

use super::Reader;

/**
 * A PCAP implementation of PacketReader
 */
pub struct PCAPReader {
    cap: pcap::Capture<pcap::Active>,
}

// This bit of magic is required for PCAPReader to be thread safe
// TODO: learn more about how this works
unsafe impl Send for PCAPReader {}
unsafe impl Sync for PCAPReader {}

// Implements the PacketReader trait for our PCAP implementation
impl Reader for PCAPReader {
    fn next_packet(&mut self) -> Result<pcap::Packet, pcap::Error> {
        self.cap.next_packet()
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
 *                 moved to and synchronized across threads.
 */
pub fn new(interface: &str) -> Arc<Mutex<Box<dyn Reader + Send + Sync>>> {
    let cap = pcap::Capture::from_device(interface)
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");
    let boxed: Box<dyn Reader + Send + Sync> = Box::new(PCAPReader { cap });
    Arc::new(Mutex::new(boxed))
}
