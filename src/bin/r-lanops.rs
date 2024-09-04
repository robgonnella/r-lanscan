use clap::Parser;
use core::time;
use itertools::Itertools;
use log::*;
use pnet::datalink::NetworkInterface;
use serde::{Deserialize, Serialize};
use simplelog;
use std::{
    collections::HashSet,
    error::Error,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
    thread,
};

use r_lanscan::{
    network, packet,
    scanners::{arp_scanner, syn_scanner, Device, Port, ScanMessage, Scanner, IDLE_TIMEOUT},
    targets, ui,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in debug mode - Only prints logs foregoing UI
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    /// Comma separated list of ports and port ranges to scan
    #[arg(short, long, default_value = "1-65535", use_value_delimiter = true)]
    ports: Vec<String>,
}

// Device with open ports
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceWithPorts {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
    pub open_ports: HashSet<Port>,
}

fn initialize_logger(args: &Args) {
    let filter = if args.debug {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Off
    };

    simplelog::TermLogger::init(
        filter,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();
}

fn process_arp(
    cidr: String,
    interface: Arc<NetworkInterface>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
) -> (Vec<Device>, Receiver<ScanMessage>) {
    let mut arp_results: HashSet<Device> = HashSet::new();

    let scanner = arp_scanner::new(
        interface,
        packet::wire::new_default_reader,
        packet::wire::new_default_sender,
        targets::ips::new(vec![cidr]),
        true,
        true,
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
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
            arp_results.insert(m.to_owned());
        }
    }

    let mut items: Vec<Device> = arp_results.into_iter().collect();
    items.sort_by_key(|i| i.ip.to_owned());

    (items, rx)
}

fn process_syn(
    interface: Arc<NetworkInterface>,
    devices: Vec<Device>,
    ports: Vec<String>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
    source_port: u16,
) -> Vec<DeviceWithPorts> {
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

    let scanner = syn_scanner::new(
        interface,
        packet::wire::new_default_reader,
        packet::wire::new_default_sender,
        Arc::new(devices),
        targets::ports::new(ports),
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
        tx,
        source_port,
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
                    d.open_ports.insert(m.open_port.to_owned());
                }
                None => {
                    warn!("received syn result for unknown device: {:?}", m);
                }
            }
        }
    }

    syn_results
}

fn monitor_network(ports: Vec<String>, data_set: Arc<RwLock<Vec<ui::app::Data>>>) {
    info!("starting network monitor");
    thread::spawn(move || {
        let interface = network::get_default_interface();
        let cidr = network::get_interface_cidr(Arc::clone(&interface));
        let source_port = network::get_available_port();
        let (tx, rx) = mpsc::channel::<ScanMessage>();
        let (arp_results, rx) = process_arp(cidr, Arc::clone(&interface), rx, tx.clone());

        let results = process_syn(
            Arc::clone(&interface),
            arp_results,
            ports.clone(),
            rx,
            tx.clone(),
            source_port,
        );

        let mut ui_data: Vec<ui::app::Data> = results
            .iter()
            .map(|d| ui::app::Data {
                hostname: d.hostname.to_owned(),
                ip: d.ip.to_owned(),
                mac: d.mac.to_owned(),
                vendor: d.vendor.to_owned(),
                ports: d
                    .open_ports
                    .iter()
                    .map(|p| p.id.to_owned())
                    .sorted()
                    .join(", "),
            })
            .collect();

        ui_data.sort_by_key(|d| d.ip.to_owned());

        {
            let mut set = data_set.write().unwrap();
            *set = ui_data;
        }

        info!("network scan completed");
        thread::sleep(time::Duration::from_secs(15));
        monitor_network(ports, Arc::clone(&data_set));
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    initialize_logger(&args);

    let mut data_set: Vec<ui::app::Data> = Vec::new();

    data_set.push(ui::app::Data {
        hostname: "scanning...".to_string(),
        ip: "".to_string(),
        mac: "".to_string(),
        vendor: "".to_string(),
        ports: "".to_string(),
    });

    let thread_safe_data_set = Arc::new(RwLock::new(data_set));

    monitor_network(args.ports, Arc::clone(&thread_safe_data_set));

    if args.debug {
        loop {}
    }

    ui::app::launch(Arc::clone(&thread_safe_data_set))
}
