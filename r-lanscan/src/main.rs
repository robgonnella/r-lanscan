use clap::Parser;
use color_eyre::eyre::{eyre, Report, Result};
use core::time;
use itertools::Itertools;
use log::*;
use prettytable;
use r_lanlib::{
    network::{self, NetworkInterface},
    packet,
    scanners::{
        arp_scanner::ARPScanner, syn_scanner::SYNScanner, Device, DeviceWithPorts, ScanError,
        ScanMessage, Scanner, IDLE_TIMEOUT,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use simplelog;
use std::{
    collections::HashSet,
    env,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

/// Local Area Network ARP and SYN scanning
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma separated list of IPs, IP ranges, and CIDR blocks to scan
    #[arg(short, long, use_value_delimiter = true)]
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

    /// Perform reverse dns lookups
    #[arg(long, default_value_t = false)]
    dns: bool,

    /// Set idle timeout in milliseconds for all scanners
    #[arg(long, default_value_t = IDLE_TIMEOUT)]
    idle_timeout_ms: u16,

    /// Choose a specific network interface for the scan
    #[arg(short, long, default_value_t = network::get_default_interface().expect("cannot find default interface").name.to_string())]
    interface: String,

    /// Sets the port for outgoing / incoming packets
    #[arg(long, default_value_t = network::get_available_port().expect("cannot find open port"))]
    source_port: u16,

    /// Prints debug logs including those from r-lanlib
    #[arg(long, default_value_t = false)]
    debug: bool,
}

fn initialize_logger(args: &Args) {
    let filter = if args.quiet {
        simplelog::LevelFilter::Error
    } else if args.debug {
        simplelog::LevelFilter::Debug
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
    info!("targets:         {:?}", args.targets);
    info!("ports            {:?}", args.ports);
    info!("json:            {}", args.json);
    info!("arpOnly:         {}", args.arp_only);
    info!("vendor:          {}", args.vendor);
    info!("dns:             {}", args.dns);
    info!("quiet:           {}", args.quiet);
    info!("idle_timeout_ms: {}", args.idle_timeout_ms);
    info!("interface:       {}", args.interface);
    info!("source_port:     {}", args.source_port);
}

fn process_arp(
    args: &Args,
    interface: &NetworkInterface,
    packet_reader: Arc<Mutex<dyn packet::Reader>>,
    packet_sender: Arc<Mutex<dyn packet::Sender>>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
) -> Result<(Vec<Device>, Receiver<ScanMessage>), ScanError> {
    let mut arp_results: HashSet<Device> = HashSet::new();

    let scanner = ARPScanner::new(
        interface,
        packet_reader,
        packet_sender,
        IPTargets::new(args.targets.clone()),
        args.vendor,
        args.dns,
        time::Duration::from_millis(args.idle_timeout_ms.into()),
        tx,
    );

    let handle = scanner.scan();

    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Done(_) => {
                debug!("scanning complete");
                break;
            }
            ScanMessage::ARPScanResult(m) => {
                debug!("received scanning message: {:?}", m);
                arp_results.insert(m.to_owned());
            }
            _ => {}
        }
    }

    handle.join().unwrap()?;

    let mut items: Vec<Device> = arp_results.into_iter().collect();
    items.sort_by_key(|i| i.ip.to_owned());

    Ok((items, rx))
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
    interface: &NetworkInterface,
    packet_reader: Arc<Mutex<dyn packet::Reader>>,
    packet_sender: Arc<Mutex<dyn packet::Sender>>,
    devices: Vec<Device>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
) -> Result<Vec<DeviceWithPorts>, ScanError> {
    let mut syn_results: Vec<DeviceWithPorts> = Vec::new();

    for d in devices.iter() {
        syn_results.push(DeviceWithPorts {
            hostname: d.hostname.to_owned(),
            ip: d.ip.to_owned(),
            mac: d.mac.to_owned(),
            vendor: d.vendor.to_owned(),
            open_ports: HashSet::new(),
        })
    }

    let scanner = SYNScanner::new(
        interface,
        packet_reader,
        packet_sender,
        devices,
        PortTargets::new(args.ports.clone()),
        args.source_port,
        time::Duration::from_millis(args.idle_timeout_ms.into()),
        tx,
    );

    let handle = scanner.scan();

    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Done(_) => {
                debug!("scanning complete");
                break;
            }
            ScanMessage::SYNScanResult(m) => {
                debug!("received scanning message: {:?}", m);
                let device = syn_results.iter_mut().find(|d| d.mac == m.device.mac);
                match device {
                    Some(d) => {
                        d.open_ports.insert(m.open_port.to_owned());
                    }
                    None => {
                        warn!("received syn result for unknown device: {:?}", m);
                    }
                }
            }
            _ => {}
        }
    }

    handle.join().unwrap()?;

    Ok(syn_results)
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
                .sorted_by_key(|p| p.id)
                .map(|p| p.id.to_owned().to_string())
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

fn is_root() -> bool {
    match env::var("USER") {
        Ok(val) => val == "root",
        Err(_e) => false,
    }
}

fn main() -> Result<(), Report> {
    color_eyre::install()?;

    let mut args = Args::parse();

    initialize_logger(&args);

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = network::get_interface(&args.interface).expect("cannot find interface");

    args.interface = interface.name.clone();

    if args.targets.len() == 0 {
        args.targets = vec![interface.cidr.clone()]
    }

    print_args(&args);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let packet_reader = packet::wire::new_default_reader(&interface).or_else(|e| {
        Err(ScanError {
            ip: None,
            port: None,
            error: Box::from(e),
        })
    })?;

    let packet_sender = packet::wire::new_default_sender(&interface).or_else(|e| {
        Err(ScanError {
            ip: None,
            port: None,
            error: Box::from(e),
        })
    })?;

    let (arp_results, rx) = process_arp(
        &args,
        &interface,
        Arc::clone(&packet_reader),
        Arc::clone(&packet_sender),
        rx,
        tx.clone(),
    )?;

    print_arp(&args, &arp_results);

    if args.arp_only {
        return Ok(());
    }

    let final_results = process_syn(
        &args,
        &interface,
        Arc::clone(&packet_reader),
        Arc::clone(&packet_sender),
        arp_results,
        rx,
        tx.clone(),
    )?;
    print_syn(&args, &final_results);

    Ok(())
}
