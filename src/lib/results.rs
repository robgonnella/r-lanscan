#[derive(Debug)]
pub struct ArpScanResult {
    pub ip: String,
    pub mac: String,
    pub vendor: String,
}

#[derive(Debug)]
pub struct SynScanResult {
    pub ip: String,
    pub mac: String,
    pub status: String,
    pub port: String,
}
