use pcap::{Active, Capture};

use crate::Scanner;

pub struct SYNScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<SYNTarget>,
}

#[derive(Debug)]
pub struct SYNTarget {
    pub ip: String,
    pub mac: String,
}

#[derive(Debug)]
pub struct SynScanResult {
    pub ip: String,
    pub mac: String,
    pub status: String,
    pub port: String,
}

impl<'a> SYNScanner<'a> {
    pub fn new(cap: &'a Capture<Active>, targets: Vec<SYNTarget>) -> SYNScanner<'a> {
        SYNScanner { cap, targets }
    }
}

impl<'a> Scanner for SYNScanner<'a> {
    fn scan(&self) {
        println!("performing SYN scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("sending SYN packet to {}", target.ip);
            // self.cap.sendpacket(buf)
        }
    }
}
