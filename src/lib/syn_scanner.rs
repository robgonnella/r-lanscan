use pcap::{Active, Capture};

use crate::{
    targets::{LazyLooper, PortTargets},
    Scanner, ScannerOptions,
};

pub struct SYNScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
    include_host_names: bool,
    include_vendor: bool,
}

#[derive(Debug)]
pub struct SYNTarget {
    pub ip: String,
    pub mac: String,
}

#[derive(Debug)]
pub struct SYNScanResult {
    pub ip: String,
    pub mac: String,
    pub status: String,
    pub port: String,
}

pub fn new<'a>(
    cap: &'a Capture<Active>,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
    options: &ScannerOptions,
) -> SYNScanner<'a> {
    SYNScanner {
        cap,
        targets,
        ports,
        include_host_names: options.include_host_names,
        include_vendor: options.include_vendor,
    }
}

impl<'a> Scanner<SYNScanResult> for SYNScanner<'a> {
    fn scan(self) -> Vec<SYNScanResult> {
        println!("performing SYN scan on targets: {:?}", self.targets);

        let results: Vec<SYNScanResult> = Vec::new();

        for target in self.targets.iter() {
            let port_list = PortTargets::new(&self.ports);

            let process_port = |port: u32| {
                println!("processing target: {}:{}", target.ip, port);
            };

            port_list.lazy_loop(process_port);
        }

        results
    }
}
