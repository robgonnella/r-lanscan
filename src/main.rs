use std::{net::Ipv4Addr, str::FromStr};

use clap::Parser;
use ipnet::Ipv4Net;
use pcap::{Capture, Device};

use r_lanscan::scanners::{full_scanner, Scanner, ScannerOptions};

fn get_default_device_name() -> String {
    let device = Device::lookup()
        .expect("device lookup failed")
        .expect("no device available");
    device.name
}

fn netmask_to_bit(netmask: &str) -> u32 {
    let bits: u32 = netmask
        .split(".")
        .map(|x| x.parse::<u8>().unwrap().count_ones())
        .sum();
    bits
}

fn get_default_network_cidr() -> Vec<String> {
    let device = Device::lookup()
        .expect("device lookup failed")
        .expect("no device available");

    let mut cidr: String = String::from("");

    for a in device.addresses.iter() {
        if a.addr.is_ipv4() && !a.addr.is_loopback() {
            let prefix = netmask_to_bit(&a.netmask.unwrap().to_string());
            let ipv4 = Ipv4Addr::from_str(a.addr.to_string().as_str()).unwrap();
            let net = Ipv4Net::new(ipv4, u8::try_from(prefix).ok().unwrap()).unwrap();
            cidr = net.trunc().to_string();
            break;
        }
    }

    vec![cidr]
}

/// Local Area Network ARP and SYN scanning
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma separated list of IPs, IP ranges, and CIDR blocks to scan
    #[arg(short, long, default_values_t = get_default_network_cidr(), use_value_delimiter = true)]
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
    #[arg(short, long, default_value_t = get_default_device_name())]
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
