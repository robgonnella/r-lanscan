use pcap::{Active, Capture};

use crate::{Scanner, ScannerOptions};

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
        options: &Option<ScannerOptions>,
    ) -> ARPScanner<'a> {
        match options {
            Some(opts) => ARPScanner {
                cap,
                targets,
                include_vendor: opts.include_vendor,
                include_hostnames: opts.include_hostnames,
            },
            None => ARPScanner {
                cap,
                targets,
                include_vendor: false,
                include_hostnames: false,
            },
        }
    }
}

impl<'a> Scanner for ARPScanner<'a> {
    fn scan(&self) {
        println!("performing ARP scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("sending ARP packet to {}", target);
            // self.cap.sendpacket(buf)
        }
    }
}
