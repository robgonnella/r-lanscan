use pcap::Active;
use pcap::Capture;

use crate::s_trait::Scanner;

pub struct FullScanner {
    targets: Vec<String>,
    cap: Capture<Active>,
}

impl FullScanner {
    pub fn new(cap: Capture<Active>, targets: Vec<String>) -> FullScanner {
        FullScanner { cap, targets }
    }
}

impl Scanner for FullScanner {
    fn scan(&self) {
        println!("performing full scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("scanning target: {}", target);
            // self.cap.sendpacket(buf)
        }
    }
}
