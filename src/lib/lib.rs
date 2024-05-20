pub trait Scanner {
    fn scan(&self);
}

mod arp_scanner;
mod full_scanner;
mod syn_scanner;

pub use arp_scanner::*;
pub use full_scanner::*;
pub use syn_scanner::*;
