use pcap::Active;
use pcap::Capture;

pub trait Scanner {
    fn scan(&self);
}

/**
 * ARPScanner
 */
#[derive(Debug)]
pub struct ArpScanResult {
    pub ip: String,
    pub mac: String,
    pub vendor: String,
}

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

/**
 * SYNScanner
 */
pub struct SynScanResult {
    pub mac: String,
    pub ip: String,
    pub status: String,
    pub port: String,
}

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

/**
 * FullScanner
 */
pub struct FullScanner {
    targets: Vec<String>,
    cap: Capture<Active>,
}

impl FullScanner {
    pub fn new(cap: Capture<Active>, targets: Vec<String>) -> FullScanner {
        FullScanner { cap, targets }
    }
}

impl Scanner for FullScanner {
    fn scan(&self) {
        println!("performing full scan on targets: {:?}", self.targets);

        for target in self.targets.iter() {
            println!("scanning target: {}", target);
            // self.cap.sendpacket(buf)
        }
    }
}
