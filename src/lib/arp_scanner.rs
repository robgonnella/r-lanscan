use pcap::Active;
use pcap::Capture;

use crate::s_trait::Scanner;

pub struct ARPScanner {
    targets: Vec<String>,
    cap: Capture<Active>,
}

impl ARPScanner {
    pub fn new(cap: Capture<Active>, targets: Vec<String>) -> ARPScanner {
        ARPScanner { cap, targets }
    }
}

impl Scanner for ARPScanner {
    fn scan(&self) {
        println!("performing ARP scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("sending ARP packet to {}", target);
            // self.cap.sendpacket(buf)
        }
    }
}
