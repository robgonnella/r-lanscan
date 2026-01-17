use mockall::mock;

use crate::scanners::Result;

use super::{Reader, Sender};

mock! {
        pub PacketReader {}
        impl Reader for PacketReader {
            fn next_packet(&mut self) -> Result<&'static [u8]>;
        }
}

mock! {
    pub PacketSender {}
    impl Sender for PacketSender {
        fn send(&mut self, packet: &[u8]) -> Result<()>;
    }
}
