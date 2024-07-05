pub trait Scanner<T> {
    fn scan(self) -> Vec<T>;
}

#[derive(Debug)]
pub struct ScannerOptions {
    pub include_vendor: bool,
    pub include_host_names: bool,
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
mod targets;
