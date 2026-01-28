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
use color_eyre::eyre::{Result, eyre};
use core::time;
use itertools::Itertools;
use r_lanlib::{
    error::Result as LibResult,
    network::{self, NetworkInterface},
    packet,
    scanners::{
        Device, IDLE_TIMEOUT, ScanMessage, Scanner, arp_scanner::ARPScanner,
        syn_scanner::SYNScanner,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{
        Arc,
        mpsc::{self, Receiver},
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
    #[arg(short, long)]
    interface: Option<String>,

    /// Sets the port for outgoing / incoming packets
    #[arg(long, default_value_t = network::get_available_port().expect("cannot find open port"))]
    source_port: u16,

    /// Prints debug logs including those from r-lanlib
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[doc(hidden)]
fn initialize_logger(args: &Args) -> Result<()> {
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
    )?;

    Ok(())
}

#[doc(hidden)]
fn print_args(args: &Args, interface: &NetworkInterface) {
    log::info!("configuration:");
    log::info!("targets:         {:?}", args.targets);
    log::info!("ports            {:?}", args.ports);
    log::info!("json:            {}", args.json);
    log::info!("arpOnly:         {}", args.arp_only);
    log::info!("vendor:          {}", args.vendor);
    log::info!("host_names:      {}", args.host_names);
    log::info!("quiet:           {}", args.quiet);
    log::info!("idle_timeout_ms: {}", args.idle_timeout_ms);
    log::info!(
        "interface:       {}",
        args.interface.as_deref().unwrap_or(&interface.name)
    );
    log::info!("cidr:            {}", interface.cidr);
    log::info!("user_ip:         {}", interface.ipv4);
    log::info!("source_port:     {}", args.source_port);
}

#[doc(hidden)]
fn process_arp(
    scanner: &dyn Scanner,
    rx: Receiver<ScanMessage>,
) -> LibResult<(Vec<Device>, Receiver<ScanMessage>)> {
    let mut arp_results: HashSet<Device> = HashSet::new();

    log::info!("starting arp scan...");

    let handle = scanner.scan()?;

    loop {
        let msg = rx.recv()?;

        match msg {
            ScanMessage::Done => {
                log::debug!("scanning complete");
                break;
            }
            ScanMessage::ARPScanDevice(m) => {
                log::debug!("received scanning message: {:?}", m);
                arp_results.insert(m.to_owned());
            }
            _ => {}
        }
    }

    handle.join()??;

    let mut items: Vec<Device> = arp_results.into_iter().collect();
    items.sort_by_key(|i| i.ip);

    Ok((items, rx))
}

#[doc(hidden)]
fn print_arp(args: &Args, devices: &Vec<Device>) -> Result<()> {
    log::info!("arp results:");

    if args.quiet && !args.arp_only {
        // only print results of SYN scanner
        return Ok(());
    }

    if args.json {
        let j: String = serde_json::to_string(&devices)?;
        println!("{}", j);
    } else {
        let mut arp_table = prettytable::Table::new();

        arp_table
            .add_row(prettytable::row!["IP", "HOSTNAME", "MAC", "VENDOR",]);

        for d in devices.iter() {
            let ip_field = if d.is_current_host {
                format!("{} [YOU]", d.ip)
            } else {
                d.ip.to_string()
            };
            arp_table.add_row(prettytable::row![
                ip_field, d.hostname, d.mac, d.vendor
            ]);
        }

        arp_table.printstd();
    }

    Ok(())
}

#[doc(hidden)]
fn process_syn(
    scanner: &dyn Scanner,
    devices: Vec<Device>,
    rx: Receiver<ScanMessage>,
) -> LibResult<HashMap<Ipv4Addr, Device>> {
    let mut syn_results: HashMap<Ipv4Addr, Device> = HashMap::new();

    for d in devices.iter() {
        syn_results.insert(d.ip, d.clone());
    }

    log::info!("starting syn scan...");

    let handle = scanner.scan()?;

    loop {
        let msg = rx.recv()?;

        match msg {
            ScanMessage::Done => {
                log::debug!("scanning complete");
                break;
            }
            ScanMessage::SYNScanDevice(device) => {
                log::debug!("received syn scanning device: {:?}", device);
                let found_device = syn_results.get_mut(&device.ip);
                match found_device {
                    Some(d) => d.open_ports.0.extend(device.open_ports.0),
                    None => {
                        log::warn!(
                            "received syn result for unknown device: {:?}",
                            device
                        );
                    }
                }
            }
            _ => {}
        }
    }

    handle.join()??;

    Ok(syn_results)
}

#[doc(hidden)]
fn print_syn(
    args: &Args,
    device_map: &HashMap<Ipv4Addr, Device>,
) -> Result<()> {
    log::info!("syn results:");

    let devices: Vec<_> = device_map.values().cloned().sorted().collect();

    if args.json {
        let j: String = serde_json::to_string(&devices)?;
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

            let ports: Vec<_> = d
                .open_ports
                .to_sorted_vec()
                .into_iter()
                .map(|p| p.to_string())
                .collect();
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

    Ok(())
}

#[doc(hidden)]
#[cfg(unix)]
fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

#[doc(hidden)]
#[cfg(windows)]
fn is_root() -> bool {
    // On Windows, check if running as Administrator
    // This is a simplified check - raw socket operations require admin privileges
    use std::process::Command;
    Command::new("net")
        .args(["session"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[doc(hidden)]
fn main() -> Result<()> {
    color_eyre::install()?;

    let mut args = Args::parse();

    initialize_logger(&args)?;

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = match &args.interface {
        Some(name) => network::get_interface(name).ok_or_else(|| {
            eyre!("Could not find network interface: {}", name)
        })?,
        None => network::get_default_interface().ok_or_else(|| {
            eyre!("Could not detect default network interface")
        })?,
    };

    args.interface = Some(interface.name.clone());

    if args.targets.is_empty() {
        args.targets = vec![interface.cidr.clone()]
    }

    print_args(&args, &interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let wire = packet::wire::default(&interface)?;

    let interface = Arc::new(interface);

    let arp = ARPScanner::builder()
        .interface(Arc::clone(&interface))
        .packet_sender(Arc::clone(&wire.0))
        .packet_reader(Arc::clone(&wire.1))
        .targets(
            IPTargets::new(args.targets.clone())
                .map_err(|e| eyre!("Invalid IP targets: {}", e))?,
        )
        .source_port(args.source_port)
        .include_vendor(args.vendor)
        .include_host_names(args.host_names)
        .idle_timeout(time::Duration::from_millis(args.idle_timeout_ms.into()))
        .notifier(tx.clone())
        .build()?;

    let (arp_results, rx) = process_arp(&arp, rx)?;

    print_arp(&args, &arp_results)?;

    if args.arp_only {
        return Ok(());
    }

    let syn = SYNScanner::builder()
        .interface(interface)
        .packet_sender(wire.0)
        .packet_reader(wire.1)
        .targets(arp_results.clone())
        .ports(
            PortTargets::new(args.ports.clone())
                .map_err(|e| eyre!("Invalid port targets: {}", e))?,
        )
        .source_port(args.source_port)
        .idle_timeout(time::Duration::from_millis(args.idle_timeout_ms.into()))
        .notifier(tx)
        .build()?;

    let final_results = process_syn(&syn, arp_results, rx)?;
    print_syn(&args, &final_results)?;

    Ok(())
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
