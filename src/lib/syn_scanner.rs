use pcap::{Active, Capture};

use crate::{
    targets::{LazyLooper, PortTargets},
    Scanner, ScannerOptions,
};

pub struct SYNScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<SYNTarget>,
    ports: Vec<String>,
    include_hostnames: bool,
    include_vendor: bool,
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
    pub fn new(
        cap: &'a Capture<Active>,
        targets: Vec<SYNTarget>,
        ports: Vec<String>,
        options: &ScannerOptions,
    ) -> SYNScanner<'a> {
        SYNScanner {
            cap,
            targets,
            ports,
            include_hostnames: options.include_hostnames,
            include_vendor: options.include_vendor,
        }
    }
}

impl<'a> Scanner for SYNScanner<'a> {
    fn scan(self) {
        println!("performing SYN scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            let port_list = PortTargets::new(&self.ports);

            let process_port = |port: u32| {
                println!("processing target: {}:{}", target.ip, port);
            };

            port_list.lazy_loop(process_port);
        }
    }
}
