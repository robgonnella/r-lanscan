use clap::Parser;
use config::{Config, ConfigManager};
use core::time;
use directories::ProjectDirs;
use log::*;
use simplelog;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};
use ui::store::{action::Action, dispatcher::Dispatcher, types::Theme};

use r_lanlib::{
    network::{self, NetworkInterface},
    packet,
    scanners::{
        arp_scanner::ARPScanner, syn_scanner::SYNScanner, Device, DeviceWithPorts, ScanError,
        ScanMessage, Scanner, IDLE_TIMEOUT,
    },
    targets::{ips::IPTargets, ports::PortTargets},
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
    #[arg(
        short,
        long,
        default_value = config::DEFAULT_PORTS_STR,
        use_value_delimiter = true
    )]
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

fn get_project_config_path() -> String {
    let project_dir = ProjectDirs::from("", "", "r-lanui").unwrap();
    let config_dir = project_dir.config_dir();
    fs::create_dir_all(config_dir).unwrap();
    config_dir.join("config.yml").to_str().unwrap().to_string()
}

fn process_arp(
    packet_reader: Arc<Mutex<dyn packet::Reader>>,
    packet_sender: Arc<Mutex<dyn packet::Sender>>,
    cidr: String,
    interface: &NetworkInterface,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
) -> Result<(Vec<Device>, Receiver<ScanMessage>), ScanError> {
    let mut arp_results: HashSet<Device> = HashSet::new();

    let scanner = ARPScanner::new(
        interface,
        packet_reader,
        packet_sender,
        IPTargets::new(vec![cidr]),
        true,
        true,
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
        tx,
    );

    let handle = scanner.scan();

    while let Ok(msg) = rx.recv() {
        if let Some(_done) = msg.done() {
            debug!("scanning complete");
            break;
        }
        if let Some(m) = msg.arp_message() {
            debug!("received scanning message: {:?}", msg);
            arp_results.insert(m.to_owned());
        }
    }

    handle.join().unwrap()?;

    let mut items: Vec<Device> = arp_results.into_iter().collect();
    items.sort_by_key(|i| i.ip.to_owned());

    Ok((items, rx))
}

fn process_syn(
    packet_reader: Arc<Mutex<dyn packet::Reader>>,
    packet_sender: Arc<Mutex<dyn packet::Sender>>,
    interface: &NetworkInterface,
    devices: Vec<Device>,
    ports: Vec<String>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
    source_port: u16,
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
        PortTargets::new(ports),
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
        tx,
        source_port,
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

fn monitor_network(
    config: Arc<Config>,
    interface: Arc<NetworkInterface>,
    dispatcher: Arc<Dispatcher>,
) -> JoinHandle<Result<(), ScanError>> {
    info!("starting network monitor");
    thread::spawn(move || -> Result<(), ScanError> {
        let source_port = network::get_available_port().or_else(|e| {
            Err(ScanError {
                ip: "".to_string(),
                port: None,
                msg: e.to_string(),
            })
        })?;

        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let packet_reader = packet::wire::new_default_reader(&interface).or_else(|e| {
            Err(ScanError {
                ip: "".to_string(),
                port: None,
                msg: e.to_string(),
            })
        })?;

        let packet_sender = packet::wire::new_default_sender(&interface).or_else(|e| {
            Err(ScanError {
                ip: "".to_string(),
                port: None,
                msg: e.to_string(),
            })
        })?;

        let (arp_results, rx) = process_arp(
            Arc::clone(&packet_reader),
            Arc::clone(&packet_sender),
            interface.cidr.clone(),
            &interface,
            rx,
            tx.clone(),
        )?;

        let results = process_syn(
            Arc::clone(&packet_reader),
            Arc::clone(&packet_sender),
            &interface,
            arp_results,
            config.ports.clone(),
            rx,
            tx.clone(),
            source_port,
        )?;

        dispatcher.dispatch(Action::UpdateDevices(&results));

        info!("network scan completed");

        thread::sleep(time::Duration::from_secs(15));
        let handle = monitor_network(config, interface, dispatcher);
        handle.join().unwrap()
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    initialize_logger(&args);

    let interface = network::get_default_interface().expect("could not get default interface");
    let config_path = get_project_config_path();
    let config_manager = Arc::new(Mutex::new(ConfigManager::new(config_path)));
    let dispatcher = Arc::new(Dispatcher::new(Arc::clone(&config_manager)));

    let manager = config_manager.lock().unwrap();
    let config: Config;
    let conf_opt = manager.get_by_cidr(&interface.cidr);
    // free up manager lock so dispatches can acquire lock as needed
    drop(manager);

    if let Some(target_config) = conf_opt {
        config = target_config;
        dispatcher.dispatch(Action::SetConfig(&config.id));
    } else {
        config = Config {
            id: fakeit::animal::animal().to_lowercase(),
            cidr: interface.cidr.clone(),
            ssh_overrides: HashMap::new(),
            ports: args.ports.clone(),
            theme: Theme::Blue.to_string(),
        };
        dispatcher.dispatch(Action::CreateAndSetConfig(&config))
    }

    // don't do anything with handle here as this call is recursive
    // so if we join our main process will never exit
    monitor_network(
        Arc::new(config),
        Arc::new(interface),
        Arc::clone(&dispatcher),
    );

    if args.debug {
        loop {}
    }

    ui::app::launch(Arc::clone(&dispatcher))
}
