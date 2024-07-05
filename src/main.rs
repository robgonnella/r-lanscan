use clap::Parser;
use pcap::Capture;

use r_lanscan::{
    network,
    scanners::{full_scanner, Scanner, ScannerOptions},
};

/// Local Area Network ARP and SYN scanning
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma separated list of IPs, IP ranges, and CIDR blocks to scan
    #[arg(short, long, default_values_t = vec![network::get_default_network_cidr()], use_value_delimiter = true)]
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

    /// Perform vendor lookups
    #[arg(long, default_value_t = false)]
    vendor: bool,

    /// Perform hostname lookups
    #[arg(long, default_value_t = false)]
    host: bool,

    /// Choose a specific network interface for the scan
    #[arg(short, long, default_value_t = network::get_default_device_name())]
    interface: String,
}

fn main() {
    let args = Args::parse();

    println!("configuration:");
    println!("  targets: {:?}", args.targets);
    println!("  ports: {:?}", args.ports);
    println!("  json: {}", args.json);
    println!("  arpOnly: {}", args.arp_only);
    println!("  vendor: {}", args.vendor);
    println!("  host: {}", args.host);
    println!("  interface: {}", args.interface);

    let interface = args.interface.as_str();

    let cap = Capture::from_device(interface)
        .expect("failed to create capture device")
        .promisc(true)
        .snaplen(65536)
        .open()
        .expect("failed to activate capture device");

    let scanner_options = &ScannerOptions {
        include_host_names: args.host,
        include_vendor: args.vendor,
    };

    let scanner = full_scanner::new(&cap, args.targets, args.ports, scanner_options);

    scanner.scan();
}
