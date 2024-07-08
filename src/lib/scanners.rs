pub trait Scanner<T> {
    fn scan(&self) -> Vec<T>;
}

pub mod arp_scanner;
pub mod full_scanner;
pub mod syn_scanner;
