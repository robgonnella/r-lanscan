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
    net::Ipv4Addr,
    str::FromStr,
    sync::{
        mpsc::{self, Receiver},
        Arc,
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
    info!("user_ip:         {}", interface.ipv4.to_string());
    info!("source_port:     {}", args.source_port);
}

fn process_arp(
    scanner: &dyn Scanner,
    rx: Receiver<ScanMessage>,
) -> Result<(Vec<Device>, Receiver<ScanMessage>), ScanError> {
    let mut arp_results: HashSet<Device> = HashSet::new();

    info!("starting arp scan...");

    let handle = scanner.scan();

    loop {
        let msg = rx.recv().or_else(|e| {
            Err(ScanError {
                ip: None,
                port: None,
                error: Box::new(e),
            })
        })?;

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
    items.sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());

    Ok((items, rx))
}

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
        let msg = rx.recv().or_else(|e| {
            Err(ScanError {
                ip: None,
                port: None,
                error: Box::new(e),
            })
        })?;

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

    print_args(&args, &interface);

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let wire = packet::wire::default(&interface).or_else(|e| {
        Err(ScanError {
            ip: None,
            port: None,
            error: Box::from(e),
        })
    })?;

    let arp = ARPScanner::new(
        &interface,
        Arc::clone(&wire.0),
        Arc::clone(&wire.1),
        IPTargets::new(args.targets.clone()),
        args.source_port,
        args.vendor,
        args.host_names,
        time::Duration::from_millis(args.idle_timeout_ms.into()),
        tx.clone(),
    );

    let (arp_results, rx) = process_arp(&arp, rx)?;

    print_arp(&args, &arp_results);

    if args.arp_only {
        return Ok(());
    }

    let syn = SYNScanner::new(
        &interface,
        wire.0,
        wire.1,
        arp_results.clone(),
        PortTargets::new(args.ports.clone()),
        args.source_port,
        time::Duration::from_millis(args.idle_timeout_ms.into()),
        tx,
    );

    let final_results = process_syn(&syn, arp_results, rx)?;
    print_syn(&args, &final_results);

    Ok(())
}

#[cfg(test)]
mod tests {
    use mockall::mock;
    use mpsc::channel;
    use pnet::util::MacAddr;
    use r_lanlib::scanners::{Port, SYNScanResult, Scanner};
    use std::{
        thread::{self, JoinHandle},
        time::Duration,
    };

    use super::*;

    mock! {
        ArpScanner{}
        impl Scanner for ArpScanner {
            fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
        }
    }

    mock! {
        SynScanner{}
        impl Scanner for SynScanner {
            fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
        }
    }

    #[test]
    fn prints_args() {
        let interface = network::get_default_interface().unwrap();

        let args = Args {
            json: false,
            arp_only: false,
            debug: false,
            host_names: true,
            idle_timeout_ms: 2000,
            interface: "interface_name".to_string(),
            ports: vec!["22".to_string()],
            quiet: false,
            source_port: 54321,
            targets: vec!["192.168.1.1".to_string()],
            vendor: true,
        };

        print_args(&args, &interface);
    }

    #[test]
    fn initializes_logger() {
        let args = Args {
            json: false,
            arp_only: false,
            debug: false,
            host_names: true,
            idle_timeout_ms: 2000,
            interface: "interface_name".to_string(),
            ports: vec!["22".to_string()],
            quiet: false,
            source_port: 54321,
            targets: vec!["192.168.1.1".to_string()],
            vendor: true,
        };

        initialize_logger(&args);
    }

    #[test]
    fn prints_arp_table_results() {
        let args = Args {
            json: false,
            arp_only: false,
            debug: false,
            host_names: true,
            idle_timeout_ms: 2000,
            interface: "interface_name".to_string(),
            ports: vec!["22".to_string()],
            quiet: false,
            source_port: 54321,
            targets: vec!["192.168.1.1".to_string()],
            vendor: true,
        };

        let device = Device {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            vendor: "vendor".to_string(),
        };

        print_arp(&args, &vec![device]);
    }

    #[test]
    fn prints_arp_json_results() {
        let args = Args {
            json: true,
            arp_only: false,
            debug: false,
            host_names: true,
            idle_timeout_ms: 2000,
            interface: "interface_name".to_string(),
            ports: vec!["22".to_string()],
            quiet: false,
            source_port: 54321,
            targets: vec!["192.168.1.1".to_string()],
            vendor: true,
        };

        let device = Device {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            vendor: "vendor".to_string(),
        };

        print_arp(&args, &vec![device]);
        assert!(true);
    }

    #[test]
    fn prints_syn_table_results() {
        let args = Args {
            json: false,
            arp_only: false,
            debug: false,
            host_names: true,
            idle_timeout_ms: 2000,
            interface: "interface_name".to_string(),
            ports: vec!["22".to_string()],
            quiet: false,
            source_port: 54321,
            targets: vec!["192.168.1.1".to_string()],
            vendor: true,
        };

        let port = Port {
            id: 22,
            service: "ssh".to_string(),
        };

        let mut open_ports = HashSet::new();
        open_ports.insert(port);

        let device = DeviceWithPorts {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            vendor: "vendor".to_string(),
            open_ports,
        };

        print_syn(&args, &vec![device]);
    }

    #[test]
    fn prints_syn_json_results() {
        let args = Args {
            json: true,
            arp_only: false,
            debug: false,
            host_names: true,
            idle_timeout_ms: 2000,
            interface: "interface_name".to_string(),
            ports: vec!["22".to_string()],
            quiet: false,
            source_port: 54321,
            targets: vec!["192.168.1.1".to_string()],
            vendor: true,
        };

        let port = Port {
            id: 22,
            service: "ssh".to_string(),
        };

        let mut open_ports = HashSet::new();
        open_ports.insert(port);

        let device = DeviceWithPorts {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            vendor: "vendor".to_string(),
            open_ports,
        };

        print_syn(&args, &vec![device]);
    }

    #[test]
    fn performs_arp_scan() {
        let mut arp = MockArpScanner::new();

        let (tx, rx) = channel();

        let device = Device {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            vendor: "vendor".to_string(),
        };

        let device_clone = device.clone();

        thread::spawn(move || {
            let _ = tx.send(ScanMessage::ARPScanResult(device_clone));
            thread::sleep(Duration::from_millis(500));
            let _ = tx.send(ScanMessage::Done(()));
        });

        arp.expect_scan().returning(|| {
            let handle: JoinHandle<Result<(), ScanError>> = thread::spawn(|| Ok(()));
            handle
        });

        let result = process_arp(&arp, rx);

        assert!(result.is_ok());

        let (devices, _) = result.unwrap();

        assert_eq!(devices[0], device);
    }

    #[test]
    fn performs_syn_scan() {
        let mut syn = MockSynScanner::new();

        let (tx, rx) = channel();

        let device = Device {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            vendor: "vendor".to_string(),
        };

        let port = Port {
            id: 22,
            service: "ssh".to_string(),
        };

        let device_clone = device.clone();
        let port_clone = port.clone();

        thread::spawn(move || {
            let _ = tx.send(ScanMessage::SYNScanResult(SYNScanResult {
                device: device_clone,
                open_port: port_clone,
            }));
            thread::sleep(Duration::from_millis(500));
            let _ = tx.send(ScanMessage::Done(()));
        });

        syn.expect_scan().returning(|| {
            let handle: JoinHandle<Result<(), ScanError>> = thread::spawn(|| Ok(()));
            handle
        });

        let result = process_syn(&syn, vec![device.clone()], rx);

        assert!(result.is_ok());

        let devices = result.unwrap();

        let mut expected_open_ports = HashSet::new();
        expected_open_ports.insert(port);

        let expected_device = DeviceWithPorts {
            hostname: device.hostname,
            ip: device.ip,
            is_current_host: device.is_current_host,
            mac: device.mac,
            vendor: device.vendor,
            open_ports: expected_open_ports,
        };

        assert_eq!(devices[0], expected_device);
    }
}
