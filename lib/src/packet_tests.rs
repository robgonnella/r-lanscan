use mockall::mock;
use std::error::Error;

use super::{Reader, Sender};

mock! {
        pub PacketReader {}
        impl Reader for PacketReader {
            fn next_packet(&mut self) -> Result<&'static [u8], Box<dyn Error>>;
        }
}

mock! {
    pub PacketSender {}
    impl Sender for PacketSender {
        fn send(&mut self, packet: &[u8]) -> Result<(), Box<dyn Error>>;
    }
}
