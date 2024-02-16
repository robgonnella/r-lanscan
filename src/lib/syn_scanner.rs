use pcap::Active;
use pcap::Capture;

use crate::results::ArpScanResult;
use crate::s_trait::Scanner;

pub struct SYNScanner {
    targets: Vec<ArpScanResult>,
    cap: Capture<Active>,
}

impl SYNScanner {
    pub fn new(cap: Capture<Active>, targets: Vec<ArpScanResult>) -> SYNScanner {
        SYNScanner { cap, targets }
    }
}

impl Scanner for SYNScanner {
    fn scan(&self) {
        println!("performing SYN scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("sending SYN packet to {}", target.ip);
            // self.cap.sendpacket(buf)
        }
    }
}
