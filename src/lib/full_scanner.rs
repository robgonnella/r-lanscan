use pcap::{Active, Capture};

use crate::{
    arp_scanner,
    syn_scanner::{self, SYNScanResult},
    Scanner, ScannerOptions,
};

pub struct FullScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    ports: Vec<String>,
    options: &'a ScannerOptions,
}

pub fn new<'a>(
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    ports: Vec<String>,
    options: &'a ScannerOptions,
) -> FullScanner<'a> {
    FullScanner {
        cap,
        targets,
        ports,
        options,
    }
}

impl<'a> Scanner<syn_scanner::SYNScanResult> for FullScanner<'a> {
    fn scan(self) -> Vec<SYNScanResult> {
        let arp = arp_scanner::new(&self.cap, self.targets, self.options);

        arp.scan();

        let syn_targets = vec![syn_scanner::SYNTarget {
            ip: String::from("192.168.68.56"),
            mac: String::from("00:00:00:00:00:00"),
        }];

        let syn = syn_scanner::new(&self.cap, syn_targets, self.ports, self.options);

        syn.scan()
    }
}
