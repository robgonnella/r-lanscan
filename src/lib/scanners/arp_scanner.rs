use pcap::{Active, Capture};

use crate::targets::{self, LazyLooper};

use super::{Scanner, ScannerOptions};

pub struct ARPScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    include_vendor: bool,
    include_host_names: bool,
}

#[derive(Debug)]
pub struct ARPScanResult {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
}

pub fn new<'a>(
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    options: &ScannerOptions,
) -> ARPScanner<'a> {
    ARPScanner {
        cap,
        targets,
        include_vendor: options.include_vendor,
        include_host_names: options.include_host_names,
    }
}

impl<'a> Scanner<ARPScanResult> for ARPScanner<'a> {
    fn scan(self) -> Vec<ARPScanResult> {
        println!("performing ARP scan on targets: {:?}", self.targets);

        let results: Vec<ARPScanResult> = Vec::new();

        let target_list = targets::ips::new(&self.targets);

        let process_target = |t: String| {
            println!("processing target: {}", t);
        };

        target_list.lazy_loop(process_target);

        results
    }
}
