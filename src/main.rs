use log::*;
use pnet::datalink::NetworkInterface;
use serde::{Deserialize, Serialize};

use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc,
};

use prettytable;

use clap::Parser;

use r_lanscan::{
    network, packet,
    scanners::{arp_scanner, syn_scanner, Device, Port, ScanMessage, Scanner},
    targets,
};
use simplelog;

// Device with open ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWithPorts {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
    pub open_ports: Vec<Port>,
}

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

    /// Only print final output nothing else
    #[arg(short, long, default_value_t = false)]
    quiet: bool,

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

#[cfg(feature = "debug_logs")]
fn initialize_logger(args: &Args) {
    let filter = if args.quiet {
        simplelog::LevelFilter::Off
    } else {
        simplelog::LevelFilter::max()
    };

    simplelog::TermLogger::init(
        filter,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();
}

#[cfg(not(feature = "debug_logs"))]
fn initialize_logger(args: &Args) {
    let filter = if args.quiet {
        simplelog::LevelFilter::Off
    } else {
        simplelog::LevelFilter::Info
    };

    simplelog::TermLogger::init(
        filter,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();
}

fn print_args(args: &Args) {
    info!("configuration:");
    info!("targets:   {:?}", args.targets);
    info!("ports      {:?}", args.ports);
    info!("json:      {}", args.json);
    info!("arpOnly:   {}", args.arp_only);
    info!("vendor:    {}", args.vendor);
    info!("host:      {}", args.host);
    info!("quiet:     {}", args.quiet);
    info!("interface: {}", args.interface);
}

fn process_arp(
    args: &Args,
    interface: Arc<NetworkInterface>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
) -> (Vec<Device>, Receiver<ScanMessage>) {
    let mut arp_results: Vec<Device> = Vec::new();

    let scanner = arp_scanner::new(
        interface,
        packet::wire::bpf::new_reader,
        packet::wire::bpf::new_sender,
        targets::ips::new(args.targets.to_owned()),
        args.vendor,
        args.host,
        tx,
    );

    scanner.scan();

    while let Ok(msg) = rx.recv() {
        if let Some(_done) = msg.is_done() {
            debug!("scanning complete");
            break;
        }
        if let Some(m) = msg.is_arp_message() {
            debug!("received scanning message: {:?}", msg);
            arp_results.push(m.to_owned());
        }
    }

    (arp_results, rx)
}

fn print_arp(args: &Args, devices: &Vec<Device>) {
    info!("arp results:");

    if args.quiet && !args.arp_only {
        return;
    }

    if args.json {
        let j: String = serde_json::to_string(&devices).unwrap();
        println!("{}", j);
    } else {
        let mut arp_table = prettytable::Table::new();

        arp_table.add_row(prettytable::row!["HOSTNAME", "IP", "MAC", "VENDOR",]);

        for d in devices.iter() {
            arp_table.add_row(prettytable::row![d.hostname, d.ip, d.mac, d.vendor]);
        }

        arp_table.printstd();
    }
}

fn process_syn(
    args: &Args,
    interface: Arc<NetworkInterface>,
    devices: Vec<Device>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
) -> Vec<DeviceWithPorts> {
    let mut syn_results: Vec<DeviceWithPorts> = Vec::new();

    for d in devices.iter() {
        syn_results.push(DeviceWithPorts {
            hostname: d.hostname.to_owned(),
            ip: d.ip.to_owned(),
            mac: d.mac.to_owned(),
            vendor: d.vendor.to_owned(),
            open_ports: vec![],
        })
    }

    let scanner = syn_scanner::new(
        interface,
        packet::wire::bpf::new_reader,
        packet::wire::bpf::new_sender,
        Arc::new(devices),
        targets::ports::new(args.ports.to_owned()),
        tx,
    );

    scanner.scan();

    while let Ok(msg) = rx.recv() {
        if let Some(_done) = msg.is_done() {
            debug!("scanning complete");
            break;
        }
        if let Some(m) = msg.is_syn_message() {
            debug!("received scanning message: {:?}", msg);
            let device = syn_results.iter_mut().find(|d| d.mac == m.device.mac);
            match device {
                Some(d) => {
                    d.open_ports.push(m.open_port.to_owned());
                }
                None => {
                    warn!("received syn result for unknown device: {:?}", m);
                }
            }
        }
    }

    syn_results
}

fn print_syn(args: &Args, devices: &Vec<DeviceWithPorts>) {
    info!("syn results:");

    if args.json {
        let j: String = serde_json::to_string(devices).unwrap();
        println!("{}", j);
    } else {
        let mut syn_table: prettytable::Table = prettytable::Table::new();

        syn_table.add_row(prettytable::row![
            "HOSTNAME",
            "IP",
            "MAC",
            "VENDOR",
            "OPEN_PORTS",
        ]);

        for d in devices {
            let ports = d
                .open_ports
                .iter()
                .map(|p| p.id.to_owned())
                .collect::<Vec<String>>();
            syn_table.add_row(prettytable::row![
                d.hostname,
                d.ip,
                d.mac,
                d.vendor,
                ports.join(", ")
            ]);
        }
        syn_table.printstd();
    }
}

fn main() {
    let args = Args::parse();

    initialize_logger(&args);

    print_args(&args);

    let interface = network::get_interface(&args.interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let (arp_results, rx) = process_arp(&args, Arc::clone(&interface), rx, tx.clone());

    print_arp(&args, &arp_results);

    if args.arp_only {
        return;
    }

    let final_results = process_syn(&args, Arc::clone(&interface), arp_results, rx, tx.clone());
    print_syn(&args, &final_results);
}
