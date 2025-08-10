//! CLI for LAN Network ARP and SYN scanning
//!
//! This is the rust version of [go-lanscan cli](https://github.com/robgonnella/go-lanscan)
//!
//! # Examples
//!
//! ```bash
//! # help menu
//! sudo r-lancli --help
//!
//! # scan network
//! sudo r-lancli
//! ```
use clap::Parser;
use color_eyre::eyre::{eyre, Report, Result};
use core::time;
use itertools::Itertools;
use log::*;
use r_lanlib::{
    network::{self, NetworkInterface},
    packet,
    scanners::{
        arp_scanner::{ARPScanner, ARPScannerArgs},
        syn_scanner::{SYNScanner, SYNScannerArgs},
        Device, DeviceWithPorts, ScanError, ScanMessage, Scanner, IDLE_TIMEOUT,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use std::{
    collections::HashSet,
    env,
    net::Ipv4Addr,
    str::FromStr,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// CLI for LAN Network ARP and SYN scanning
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
    host_names: bool,

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

#[doc(hidden)]
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

#[doc(hidden)]
fn print_args(args: &Args, interface: &NetworkInterface) {
    info!("configuration:");
    info!("targets:         {:?}", args.targets);
    info!("ports            {:?}", args.ports);
    info!("json:            {}", args.json);
    info!("arpOnly:         {}", args.arp_only);
    info!("vendor:          {}", args.vendor);
    info!("host_names:      {}", args.host_names);
    info!("quiet:           {}", args.quiet);
    info!("idle_timeout_ms: {}", args.idle_timeout_ms);
    info!("interface:       {}", interface.name);
    info!("cidr:            {}", interface.cidr);
    info!("user_ip:         {}", interface.ipv4);
    info!("source_port:     {}", args.source_port);
}

#[doc(hidden)]
fn process_arp(
    scanner: &dyn Scanner,
    rx: Receiver<ScanMessage>,
) -> Result<(Vec<Device>, Receiver<ScanMessage>), ScanError> {
    let mut arp_results: HashSet<Device> = HashSet::new();

    info!("starting arp scan...");

    let handle = scanner.scan();

    loop {
        let msg = rx.recv().map_err(|e| ScanError {
            ip: None,
            port: None,
            error: Box::new(e),
        })?;

        match msg {
            ScanMessage::Done => {
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
    items.sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());

    Ok((items, rx))
}

#[doc(hidden)]
fn print_arp(args: &Args, devices: &Vec<Device>) {
    info!("arp results:");

    if args.quiet && !args.arp_only {
        // only print results of SYN scanner
        return;
    }

    if args.json {
        let j: String = serde_json::to_string(&devices).unwrap();
        println!("{}", j);
    } else {
        let mut arp_table = prettytable::Table::new();

        arp_table.add_row(prettytable::row!["IP", "HOSTNAME", "MAC", "VENDOR",]);

        for d in devices.iter() {
            let ip_field = if d.is_current_host {
                format!("{} [YOU]", d.ip)
            } else {
                d.ip.to_string()
            };
            arp_table.add_row(prettytable::row![ip_field, d.hostname, d.mac, d.vendor]);
        }

        arp_table.printstd();
    }
}

#[doc(hidden)]
fn process_syn(
    scanner: &dyn Scanner,
    devices: Vec<Device>,
    rx: Receiver<ScanMessage>,
) -> Result<Vec<DeviceWithPorts>, ScanError> {
    let mut syn_results: Vec<DeviceWithPorts> = Vec::new();

    for d in devices.iter() {
        syn_results.push(DeviceWithPorts {
            hostname: d.hostname.to_owned(),
            ip: d.ip.to_owned(),
            mac: d.mac.to_owned(),
            vendor: d.vendor.to_owned(),
            is_current_host: d.is_current_host,
            open_ports: HashSet::new(),
        })
    }

    info!("starting syn scan...");

    let handle = scanner.scan();

    loop {
        let msg = rx.recv().map_err(|e| ScanError {
            ip: None,
            port: None,
            error: Box::new(e),
        })?;

        match msg {
            ScanMessage::Done => {
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

#[doc(hidden)]
fn print_syn(args: &Args, devices: &Vec<DeviceWithPorts>) {
    info!("syn results:");

    if args.json {
        let j: String = serde_json::to_string(devices).unwrap();
        println!("{}", j);
    } else {
        let mut syn_table: prettytable::Table = prettytable::Table::new();

        syn_table.add_row(prettytable::row![
            "IP",
            "HOSTNAME",
            "MAC",
            "VENDOR",
            "OPEN_PORTS",
        ]);

        for d in devices {
            let ip_field = if d.is_current_host {
                format!("{} [YOU]", d.ip)
            } else {
                d.ip.to_string()
            };

            let ports = d
                .open_ports
                .iter()
                .sorted_by_key(|p| p.id)
                .map(|p| p.id.to_owned().to_string())
                .collect::<Vec<String>>();
            syn_table.add_row(prettytable::row![
                ip_field,
                d.hostname,
                d.mac,
                d.vendor,
                ports.join(", ")
            ]);
        }
        syn_table.printstd();
    }
}

#[doc(hidden)]
fn is_root() -> bool {
    match env::var("USER") {
        Ok(val) => val == "root",
        Err(_e) => false,
    }
}

#[doc(hidden)]
fn main() -> Result<(), Report> {
    color_eyre::install()?;

    let mut args = Args::parse();

    initialize_logger(&args);

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = network::get_interface(&args.interface).expect("cannot find interface");

    args.interface = interface.name.clone();

    if args.targets.is_empty() {
        args.targets = vec![interface.cidr.clone()]
    }

    print_args(&args, &interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let wire = packet::wire::default(&interface).map_err(|e| ScanError {
        ip: None,
        port: None,
        error: e,
    })?;

    let arp = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: Arc::clone(&wire.0),
        packet_sender: Arc::clone(&wire.1),
        targets: IPTargets::new(args.targets.clone()),
        source_port: args.source_port,
        include_vendor: args.vendor,
        include_host_names: args.host_names,
        idle_timeout: time::Duration::from_millis(args.idle_timeout_ms.into()),
        notifier: tx.clone(),
    });

    let (arp_results, rx) = process_arp(&arp, rx)?;

    print_arp(&args, &arp_results);

    if args.arp_only {
        return Ok(());
    }

    let syn = SYNScanner::new(SYNScannerArgs {
        interface: &interface,
        packet_reader: wire.0,
        packet_sender: wire.1,
        targets: arp_results.clone(),
        ports: PortTargets::new(args.ports.clone()),
        source_port: args.source_port,
        idle_timeout: time::Duration::from_millis(args.idle_timeout_ms.into()),
        notifier: tx,
    });

    let final_results = process_syn(&syn, arp_results, rx)?;
    print_syn(&args, &final_results);

    Ok(())
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
