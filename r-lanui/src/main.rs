use clap::Parser;
use color_eyre::eyre::{eyre, Report};
use config::{Config, ConfigManager};
use core::time;
use directories::ProjectDirs;
use log::*;
use simplelog;
use std::{
    collections::HashSet,
    env, fs,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};
use ui::store::{action::Action, dispatcher::Dispatcher};

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
    interface: &NetworkInterface,
    cidr: String,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
    dispatcher: Arc<Dispatcher>,
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

    dispatcher.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing ARP Scan…",
    ))));

    let handle = scanner.scan();

    while let Ok(msg) = rx.recv() {
        if let Some(_done) = msg.done() {
            debug!("scanning complete");
            break;
        }
        if let Some(m) = msg.arp_message() {
            debug!("received scanning message: {:?}", msg);
            arp_results.insert(m.to_owned());
            dispatcher.dispatch(Action::AddDevice(&DeviceWithPorts {
                hostname: m.hostname.clone(),
                ip: m.ip.clone(),
                mac: m.mac.clone(),
                open_ports: HashSet::new(),
                vendor: m.vendor.clone(),
            }));
        }
    }

    handle.join().unwrap()?;

    let items: Vec<Device> = arp_results.into_iter().collect();

    dispatcher.dispatch(Action::UpdateMessage(None));

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
    dispatcher: Arc<Dispatcher>,
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
        source_port,
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
        tx,
    );

    dispatcher.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing SYN Scan…",
    ))));

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
                        dispatcher.dispatch(Action::AddDevice(d));
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

    dispatcher.dispatch(Action::UpdateMessage(None));

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
                ip: None,
                port: None,
                error: Box::from(e),
            })
        })?;

        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let wire = packet::wire::default(&interface).or_else(|e| {
            Err(ScanError {
                ip: None,
                port: None,
                error: Box::from(e),
            })
        })?;

        let (arp_results, rx) = process_arp(
            Arc::clone(&wire.0),
            Arc::clone(&wire.1),
            &interface,
            interface.cidr.clone(),
            rx,
            tx.clone(),
            Arc::clone(&dispatcher),
        )?;

        let results = process_syn(
            Arc::clone(&wire.0),
            Arc::clone(&wire.1),
            &interface,
            arp_results,
            config.ports.clone(),
            rx,
            tx.clone(),
            source_port,
            Arc::clone(&dispatcher),
        )?;

        dispatcher.dispatch(Action::UpdateAllDevices(&results));

        info!("network scan completed");

        thread::sleep(time::Duration::from_secs(15));
        let handle = monitor_network(config, interface, dispatcher);
        handle.join().unwrap()
    })
}

fn init(args: &Args, interface: &NetworkInterface) -> (Config, Arc<Dispatcher>) {
    let config_path = get_project_config_path();
    let config_manager = Arc::new(Mutex::new(ConfigManager::new(&config_path)));
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
            ports: args.ports.clone(),
            ..Config::default()
        };
        dispatcher.dispatch(Action::CreateAndSetConfig(&config))
    }

    (config, dispatcher)
}

fn is_root() -> bool {
    match env::var("USER") {
        Ok(val) => val == "root",
        Err(_e) => false,
    }
}

fn main() -> Result<(), Report> {
    color_eyre::install()?;

    let args = Args::parse();

    initialize_logger(&args);

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = network::get_default_interface().expect("could not get default interface");
    let (config, dispatcher) = init(&args, &interface);

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
