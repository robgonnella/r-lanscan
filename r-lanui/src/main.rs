use clap::Parser;
use config::{Config, ConfigManager};
use core::time;
use directories::ProjectDirs;
use log::*;
use pnet::datalink::NetworkInterface;
use simplelog;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};
use ui::store::{action::Action, dispatcher::Dispatcher, types::Theme};

use r_lanlib::{
    network, packet,
    scanners::{
        arp_scanner, syn_scanner, Device, DeviceWithPorts, ScanMessage, Scanner, IDLE_TIMEOUT,
    },
    targets,
};

mod config;
mod ui;

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

fn monitor_network(
    ports: Vec<String>,
    interface: Arc<NetworkInterface>,
    cidr: String,
    dispatcher: Arc<Dispatcher>,
) {
    info!("starting network monitor");
    thread::spawn(move || {
        let source_port = network::get_available_port();
        let (tx, rx) = mpsc::channel::<ScanMessage>();
        let (arp_results, rx) = process_arp(cidr.clone(), Arc::clone(&interface), rx, tx.clone());

        let results = process_syn(
            Arc::clone(&interface),
            arp_results,
            ports.clone(),
            rx,
            tx.clone(),
            source_port,
        );

        dispatcher.dispatch(Action::UpdateDevices(results));

        info!("network scan completed");
        thread::sleep(time::Duration::from_secs(15));
        monitor_network(ports, Arc::clone(&interface), cidr, dispatcher);
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    initialize_logger(&args);

    let interface = network::get_default_interface();
    let cidr = network::get_interface_cidr(Arc::clone(&interface));

    let project_dir = ProjectDirs::from("", "", "r-lanui").unwrap();
    let config_dir = project_dir.config_dir();
    fs::create_dir_all(config_dir).unwrap();
    let config_path = config_dir.join("config.yml").to_str().unwrap().to_string();
    let config_manager = Arc::new(Mutex::new(ConfigManager::new(config_path)));
    let dispatcher = Arc::new(Dispatcher::new(Arc::clone(&config_manager)));

    {
        let manager = config_manager.lock().unwrap();
        let config = manager.get_by_cidr(cidr.clone());
        drop(manager);

        if let Some(target_config) = config {
            dispatcher.dispatch(Action::SetConfig(target_config.id));
        } else {
            let config = Config {
                id: fakeit::animal::animal().to_lowercase(),
                cidr: cidr.clone(),
                ssh_overrides: HashMap::new(),
                theme: Theme::Blue.to_string(),
            };
            dispatcher.dispatch(Action::CreateAndSetConfig(config))
        }
    }

    monitor_network(
        args.ports,
        Arc::clone(&interface),
        cidr,
        Arc::clone(&dispatcher),
    );

    if args.debug {
        loop {}
    }

    ui::app::launch(Arc::clone(&dispatcher))
}
