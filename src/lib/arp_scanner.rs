use pcap::{Active, Capture};

use crate::{
    targets::{IPTargets, LazyLooper},
    Scanner, ScannerOptions,
};

pub struct ARPScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    include_vendor: bool,
    include_hostnames: bool,
}

#[derive(Debug)]
pub struct ArpScanResult {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
}

impl<'a> ARPScanner<'a> {
    pub fn new(
        cap: &'a Capture<Active>,
        targets: Vec<String>,
        options: &ScannerOptions,
    ) -> ARPScanner<'a> {
        ARPScanner {
            cap,
            targets,
            include_vendor: options.include_vendor,
            include_hostnames: options.include_hostnames,
        }
    }
}

impl<'a> Scanner for ARPScanner<'a> {
    fn scan(self) {
        println!("performing ARP scan on targets: {:?}", self.targets);

        let target_list = IPTargets::new(&self.targets);

        let process_target = |t: String| {
            println!("processing target: {}", t);
        };

        target_list.lazy_loop(process_target);
    }
}
