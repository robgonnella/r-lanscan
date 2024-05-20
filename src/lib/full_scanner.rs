use pcap::{Active, Capture};

use crate::{Scanner, ScannerOptions};

pub struct FullScanner<'a> {
    cap: &'a Capture<Active>,
    targets: Vec<String>,
    include_vendor: bool,
    include_hostnames: bool,
}

impl<'a> FullScanner<'a> {
    pub fn new(
        cap: &'a Capture<Active>,
        targets: Vec<String>,
        options: &Option<ScannerOptions>,
    ) -> FullScanner<'a> {
        match options {
            Some(opts) => FullScanner {
                cap,
                targets,
                include_vendor: opts.include_vendor,
                include_hostnames: opts.include_hostnames,
            },
            None => FullScanner {
                cap,
                targets,
                include_vendor: false,
                include_hostnames: false,
            },
        }
    }
}

impl<'a> Scanner for FullScanner<'a> {
    fn scan(&self) {
        println!("performing full scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("scanning target: {}", target);
            // self.cap.sendpacket(buf)
        }
    }
}
