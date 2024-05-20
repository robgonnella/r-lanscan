use pcap::{Active, Capture};

use crate::Scanner;

pub struct FullScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<String>,
}

impl<'a> FullScanner<'a> {
    pub fn new(cap: &'a Capture<Active>, targets: Vec<String>) -> FullScanner<'a> {
        FullScanner { cap, targets }
    }
}

impl<'a> Scanner for FullScanner<'a> {
    fn scan(&self) {
        println!("performing full scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("scanning target: {}", target);
            // self.cap.sendpacket(buf)
        }
    }
}
