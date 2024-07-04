pub trait Scanner {
    fn scan(self);
}

#[derive(Debug)]
pub struct ScannerOptions {
    pub include_vendor: bool,
    pub include_hostnames: bool,
}

mod arp_scanner;
mod full_scanner;
mod syn_scanner;
mod targets;

pub use arp_scanner::*;
pub use full_scanner::*;
pub use syn_scanner::*;
