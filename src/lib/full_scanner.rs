use pcap::{Active, Capture};

use crate::{ARPScanner, SYNScanner, SYNTarget, Scanner, ScannerOptions};

pub struct FullScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    ports: Vec<String>,
    options: &'a ScannerOptions,
}

impl<'a> FullScanner<'a> {
    pub fn new(
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
}

impl<'a> Scanner for FullScanner<'a> {
    fn scan(self) {
        let arp_scanner = ARPScanner::new(&self.cap, self.targets, self.options);

        arp_scanner.scan();

        let syn_targets: Vec<SYNTarget> = vec![SYNTarget {
            ip: String::from("192.168.68.56"),
            mac: String::from("00:00:00:00:00:00"),
        }];

        let syn_scanner = SYNScanner::new(&self.cap, syn_targets, self.ports, self.options);

        syn_scanner.scan();
    }
}
