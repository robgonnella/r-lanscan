use core::time;
use std::error::Error;

pub mod arp;
pub mod heartbeat;
pub mod rst;
pub mod syn;
pub mod wire;

/// Default timing for throttling packet sends to prevent packet loss
pub const DEFAULT_PACKET_SEND_TIMING: time::Duration = time::Duration::from_micros(50);

pub trait Reader: Send + Sync {
    fn next_packet(&mut self) -> Result<&[u8], Box<dyn Error>>;
}

pub trait Sender: Send + Sync {
    fn send(&mut self, packet: &[u8]) -> Result<(), Box<dyn Error>>;
}

#[cfg(test)]
pub mod mocks {
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
}
