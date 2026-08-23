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
use color_eyre::eyre::{Result, WrapErr, eyre};
use core::time;
use itertools::Itertools;
use r_lanlib::{
    error::Result as LibResult,
    network::{self, NetworkInterface, get_default_gateway},
    oui,
    scanners::{
        Device, IDLE_TIMEOUT, ScanMessage, Scanner, arp_scanner::ARPScanner,
        syn_scanner::SYNScanner,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    time::Duration,
};

// 30 days
const OUI_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 30);

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

    /// Packet send throttle. Increasing throttle duration will result
    /// in more accurate scans and latency calculations at the expense
    /// of slower scans
    #[arg(long, value_parser = humantime::parse_duration, default_value = "200µs")]
    throttle: Duration,

    /// Skips ARP scanning and instead uses json output from previous arp scan
    /// to seed the list of devices for port scanning
    #[arg(long, conflicts_with_all = ["arp_only", "targets"])]
    from_devices_json: Option<PathBuf>,

    /// Prints debug logs including those from r-lanlib
    #[arg(long, default_value_t = false)]
    debug: bool,
}

/// Chooses the log level for a run. `--json` implies quiet unless `--debug`
/// was also passed, since info logs go to stdout and would otherwise corrupt
/// a redirected json report.
fn log_filter(args: &Args) -> simplelog::LevelFilter {
    if args.quiet || (args.json && !args.debug) {
        simplelog::LevelFilter::Error
    } else if args.debug {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Info
    }
}

fn initialize_logger(args: &Args) -> Result<()> {
    simplelog::TermLogger::init(
        log_filter(args),
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    Ok(())
}

fn print_args(args: &Args, interface: &NetworkInterface) {
    log::info!("configuration:");
    log::info!("targets:           {:?}", args.targets);
    log::info!("ports:             {:?}", args.ports);
    log::info!("json:              {}", args.json);
    log::info!(
        "from_devices_json: {}",
        args.from_devices_json
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    log::info!("arpOnly:           {}", args.arp_only);
    log::info!("vendor:            {}", args.vendor);
    log::info!("host_names:        {}", args.host_names);
    log::info!("quiet:             {}", args.quiet);
    log::info!("idle_timeout_ms:   {}", args.idle_timeout_ms);
    log::info!(
        "interface:         {}",
        args.interface.as_deref().unwrap_or(&interface.name)
    );
    log::info!("cidr:              {}", interface.cidr);
    log::info!("user_ip:           {}", interface.ipv4);
    log::info!("source_port:       {}", args.source_port);
    log::info!("throttle:          {:?}", args.throttle);
}

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

/// Whether the arp report should be printed. When a SYN scan is going to
/// follow, the syn report is a strict superset of the arp one (`process_syn`
/// seeds itself from every arp device), so printing both would emit two json
/// arrays to stdout and leave a redirected report unparseable.
fn should_print_arp_report(args: &Args) -> bool {
    args.arp_only || !(args.quiet || args.json)
}

fn print_arp(args: &Args, devices: &Vec<Device>) -> Result<()> {
    log::info!("arp results:");

    if !should_print_arp_report(args) {
        // only print results of SYN scanner
        return Ok(());
    }

    if args.json {
        let j: String = serde_json::to_string(&devices)?;
        println!("{}", j);
    } else {
        let mut arp_table = prettytable::Table::new();

        arp_table.add_row(prettytable::row![
            "IP", "HOSTNAME", "MAC", "VENDOR", "LATENCY",
        ]);

        for d in devices.iter() {
            let ip_field = if d.is_current_host {
                format!("{} [YOU]", d.ip)
            } else if d.is_gateway {
                format!("{} [GTWY]", d.ip)
            } else {
                d.ip.to_string()
            };
            let latency = d
                .latency_ms
                .map(|ms| format!("{}ms", ms))
                .unwrap_or_default();
            arp_table.add_row(prettytable::row![
                ip_field, d.hostname, d.mac, d.vendor, latency
            ]);
        }

        arp_table.printstd();
    }

    Ok(())
}

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
            "LATENCY",
            "OPEN_PORTS",
        ]);

        for d in devices {
            let ip_field = if d.is_current_host {
                format!("{} [YOU]", d.ip)
            } else if d.is_gateway {
                format!("{} [GTWY]", d.ip)
            } else {
                d.ip.to_string()
            };

            let latency = d
                .latency_ms
                .map(|ms| format!("{}ms", ms))
                .unwrap_or_default();

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
                latency,
                ports.join(", ")
            ]);
        }
        syn_table.printstd();
    }

    Ok(())
}

#[cfg(unix)]
fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

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

/// Loads a device list from the json output of a previous ARP scan, to be
/// used as the seed for port scanning in place of a fresh ARP scan.
fn load_devices_from_json(filepath: &Path) -> Result<Vec<Device>> {
    let file_content = fs::read_to_string(filepath).wrap_err_with(|| {
        format!("failed to read arp json file: {}", filepath.display())
    })?;

    let devices: Vec<Device> = serde_json::from_str(&file_content)
        .wrap_err_with(|| {
            format!("failed to parse arp json file: {}", filepath.display())
        })?;

    if devices.is_empty() {
        return Err(eyre!(
            "no devices found in arp json file: {}",
            filepath.display()
        ));
    }

    Ok(devices)
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut args = Args::parse();

    initialize_logger(&args)?;

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = match &args.interface {
        Some(name) => network::get_interface(name)?,
        None => network::get_default_interface()?,
    };

    args.interface = Some(interface.name.clone());

    if args.targets.is_empty() {
        args.targets = vec![interface.cidr.clone()]
    }

    print_args(&args, &interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let wire = r_lanlib::wire::default(&interface)?;

    let interface = Arc::new(interface);

    let oui = if args.vendor {
        Some(oui::default("r-lanscan", OUI_MAX_AGE)?)
    } else {
        None
    };

    let (arp_results, rx) = if let Some(filepath) = &args.from_devices_json {
        let devices = load_devices_from_json(filepath)?;
        (devices, rx)
    } else {
        let arp = ARPScanner::builder()
            .interface(Arc::clone(&interface))
            .wire(wire.clone())
            .gateway(get_default_gateway())
            .targets(
                IPTargets::new(args.targets.clone())
                    .map_err(|e| eyre!("Invalid IP targets: {}", e))?,
            )
            .source_port(args.source_port)
            .include_vendor(args.vendor)
            .include_host_names(args.host_names)
            .idle_timeout(time::Duration::from_millis(
                args.idle_timeout_ms.into(),
            ))
            .notifier(tx.clone())
            .throttle(args.throttle)
            .oui(oui)
            .build()?;

        let (arp_results, rx) = process_arp(&arp, rx)?;

        print_arp(&args, &arp_results)?;

        (arp_results, rx)
    };

    if args.arp_only {
        return Ok(());
    }

    let syn = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(arp_results.clone())
        .ports(
            PortTargets::new(args.ports.clone())
                .map_err(|e| eyre!("Invalid port targets: {}", e))?,
        )
        .source_port(args.source_port)
        .idle_timeout(time::Duration::from_millis(args.idle_timeout_ms.into()))
        .notifier(tx)
        .throttle(args.throttle)
        .build()?;

    let final_results = process_syn(&syn, arp_results, rx)?;
    print_syn(&args, &final_results)?;

    Ok(())
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
