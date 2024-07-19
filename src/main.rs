use log::*;

use std::sync::mpsc;

use clap::Parser;

use r_lanscan::{
    network, packet,
    scanners::{arp_scanner, full_scanner, Device, SYNScanResult, ScanMessage, Scanner},
    targets,
};
use simplelog;

/// Local Area Network ARP and SYN scanning
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma separated list of IPs, IP ranges, and CIDR blocks to scan
    #[arg(short, long, default_values_t = vec![network::get_interface_cidr(network::get_default_interface())], use_value_delimiter = true)]
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
    #[arg(short, long, default_value_t = network::get_default_interface().name.to_string())]
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

    let interface = network::get_interface(&args.interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    if args.arp_only {
        let scanner = arp_scanner::new(
            interface,
            packet::wire::bpf::new_reader,
            packet::wire::bpf::new_sender,
            targets::ips::new(args.targets),
            args.vendor,
            args.host,
            tx.clone(),
        );

        scanner.scan();

        let mut results: Vec<Device> = Vec::new();

        while let Ok(msg) = rx.recv() {
            if let Some(_done) = msg.is_done() {
                info!("scanning complete");
                break;
            }
            if let Some(m) = msg.is_arp_message() {
                info!("received scanning message: {:?}", msg);
                results.push(m.to_owned());
            }
        }

        info!("scan results: {:?}", results);
    } else {
        let scanner = full_scanner::new(
            interface,
            packet::wire::bpf::new_reader,
            packet::wire::bpf::new_sender,
            targets::ips::new(args.targets),
            targets::ports::new(args.ports),
            args.vendor,
            args.host,
            tx.clone(),
        );

        scanner.scan();

        let mut results: Vec<SYNScanResult> = Vec::new();

        while let Ok(msg) = rx.recv() {
            if let Some(_done) = msg.is_done() {
                info!("scanning complete");
                break;
            }
            if let Some(m) = msg.is_syn_message() {
                info!("received scanning message: {:?}", msg);
                results.push(SYNScanResult {
                    device: m.device.to_owned(),
                    port: m.port.to_owned(),
                    port_service: m.port_service.to_owned(),
                    port_status: m.port_status.to_owned(),
                });
            }
        }

        info!("scan results: {:?}", results);
    }
}
