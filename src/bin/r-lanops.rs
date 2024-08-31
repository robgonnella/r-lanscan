use clap::Parser;
use core::time;
use log::*;
use ops::ui;
use pnet::datalink::NetworkInterface;
use serde::{Deserialize, Serialize};
use simplelog;
use std::{
    collections::HashSet,
    error::Error,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

mod ops;

use r_lanscan::{
    network, packet,
    scanners::{arp_scanner, syn_scanner, Device, Port, ScanMessage, Scanner, IDLE_TIMEOUT},
    targets,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in debug mode - Only prints logs foregoing UI
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

// Device with open ports
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceWithPorts {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub vendor: String,
    pub open_ports: Vec<Port>,
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
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
    source_port: u16,
) {
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
        packet::wire::new_default_reader,
        packet::wire::new_default_sender,
        Arc::new(devices),
        targets::ports::new(vec!["1-65535".to_string()]),
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
                    d.open_ports.push(m.open_port.to_owned());
                    d.open_ports.sort_by_key(|p| p.id.to_owned())
                }
                None => {
                    warn!("received syn result for unknown device: {:?}", m);
                }
            }
        }
    }
}

fn monitor_network() {
    thread::spawn(|| {
        let interface = network::get_default_interface();
        let cidr = network::get_interface_cidr(Arc::clone(&interface));
        let source_port = network::get_available_port();
        let (tx, rx) = mpsc::channel::<ScanMessage>();
        let (arp_results, rx) = process_arp(cidr, Arc::clone(&interface), rx, tx.clone());

        process_syn(
            Arc::clone(&interface),
            arp_results,
            rx,
            tx.clone(),
            source_port,
        );
        thread::sleep(time::Duration::from_secs(15));
        monitor_network();
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    initialize_logger(&args);

    if args.debug {
        monitor_network();
        loop {}
    }

    ui::launch()
}
