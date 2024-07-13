use log::*;

use std::sync::mpsc;

use clap::Parser;

use r_lanscan::{
    network, packet,
    scanners::{full_scanner, ScanMessage, Scanner},
};
use simplelog;

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

fn initialize_logger() {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::max(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();
}

fn main() {
    initialize_logger();

    let args = Args::parse();

    info!("configuration:");
    info!("  targets: {:?}", args.targets);
    info!("  ports: {:?}", args.ports);
    info!("  json: {}", args.json);
    info!("  arpOnly: {}", args.arp_only);
    info!("  vendor: {}", args.vendor);
    info!("  host: {}", args.host);
    info!("  interface: {}", args.interface);

    let interface = args.interface.as_str();

    let reader = packet::pcap_reader::new(interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let scanner = full_scanner::new(
        reader,
        args.targets,
        args.ports,
        args.vendor,
        args.host,
        tx.clone(),
    );

    scanner.scan();

    while let Ok(msg) = rx.recv() {
        if msg.is_done() {
            info!("scanning complete");
            break;
        }
        info!("received scanning message: {:?}", msg);
    }
}
