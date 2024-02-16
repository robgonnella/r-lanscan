use clap::Parser;
use pcap::Capture;
use pcap::Device;

use crate::scanner::Scanner;

#[path = "scanner/scanner.rs"]
mod scanner;

fn get_default_device() -> String {
    let device = Device::lookup()
        .expect("device lookup failed")
        .expect("no device available");
    device.name
}

/// Local Area Network ARP and SYN scanning
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma separated list of IPs, IP ranges, and CIDR blocks to scan
    #[arg(short, long, default_value = "", use_value_delimiter = true)]
    targets: Vec<String>,

    /// Comma separated list of ports and port ranges to scan
    #[arg(short, long, default_value = "1-65535", use_value_delimiter = true)]
    ports: Vec<String>,

    /// Output final report in json instead of table text
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Perform only an ARP scan (omits SYN scanning)
    #[arg(long, default_value_t = false)]
    arp_only: bool,

    /// Choose a specific network interface for the scan
    #[arg(short, long, default_value_t = get_default_device())]
    interface: String,
}

fn main() {
    let args = Args::parse();

    println!("configuration:");
    println!("  targets: {:?}", args.targets);
    println!("  ports: {:?}", args.ports);
    println!("  json: {}", args.json);
    println!("  arpOnly: {}", args.arp_only);
    println!("  interface: {}", args.interface);

    let cap1 = Capture::from_device(args.interface.as_str())
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");

    let cap2 = Capture::from_device(args.interface.as_str())
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");

    let cap3 = Capture::from_device(args.interface.as_str())
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");

    let arp_targets = args.targets.clone();

    let arp_scanner = scanner::ARPScanner::new(cap1, arp_targets);

    arp_scanner.scan();

    let syn_targets: Vec<scanner::ArpScanResult> = vec![scanner::ArpScanResult {
        ip: String::from("192.168.68.56"),
        mac: String::from("00:00:00:00:00:00"),
        vendor: String::from("macOS"),
    }];

    let syn_scanner = scanner::SYNScanner::new(cap2, syn_targets);

    syn_scanner.scan();

    let full_targets = args.targets.clone();

    let full_scanner = scanner::FullScanner::new(cap3, full_targets);

    full_scanner.scan();
}
