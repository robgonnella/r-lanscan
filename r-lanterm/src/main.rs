use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use config::{Config, ConfigManager};
use core::time;
use directories::ProjectDirs;
use log::*;
use r_lanlib::{
    network::{self, NetworkInterface},
    packet,
    scanners::{
        arp_scanner::ARPScanner, syn_scanner::SYNScanner, DeviceWithPorts, ScanError, ScanMessage,
        Scanner, IDLE_TIMEOUT,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use simplelog;
use std::{
    collections::HashSet,
    env, fs,
    sync::{
        mpsc::{self, channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use ui::{
    app, events,
    store::{action::Action, derived::get_detected_devices, store::Store},
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
    source_port: u16,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
    store: Arc<Store>,
) -> Result<Receiver<ScanMessage>, ScanError> {
    let scanner = ARPScanner::new(
        interface,
        packet_reader,
        packet_sender,
        IPTargets::new(vec![cidr]),
        source_port,
        true,
        true,
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
        tx,
    );

    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing ARP Scan…",
    ))));

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
                store.dispatch(Action::AddDevice(DeviceWithPorts {
                    hostname: m.hostname.clone(),
                    ip: m.ip.clone(),
                    mac: m.mac.clone(),
                    open_ports: HashSet::new(),
                    vendor: m.vendor.clone(),
                    is_current_host: m.is_current_host,
                }));
            }
            _ => {}
        }
    }

    debug!("waiting for arp handle to finish");

    handle.join().unwrap()?;

    store.dispatch(Action::UpdateMessage(None));

    debug!("finished arp scan");
    Ok(rx)
}

fn process_syn(
    packet_reader: Arc<Mutex<dyn packet::Reader>>,
    packet_sender: Arc<Mutex<dyn packet::Sender>>,
    interface: &NetworkInterface,
    ports: Vec<String>,
    rx: Receiver<ScanMessage>,
    tx: Sender<ScanMessage>,
    source_port: u16,
    store: Arc<Store>,
) -> Result<Vec<DeviceWithPorts>, ScanError> {
    let state = store.get_state();
    let arp_devices = get_detected_devices(&state);
    let mut syn_results: Vec<DeviceWithPorts> = Vec::new();

    for d in arp_devices.iter() {
        syn_results.push(DeviceWithPorts {
            hostname: d.hostname.to_owned(),
            ip: d.ip.to_owned(),
            mac: d.mac.to_owned(),
            vendor: d.vendor.to_owned(),
            is_current_host: d.is_current_host,
            open_ports: HashSet::new(),
        })
    }

    let scanner = SYNScanner::new(
        interface,
        packet_reader,
        packet_sender,
        arp_devices,
        PortTargets::new(ports),
        source_port,
        time::Duration::from_millis(IDLE_TIMEOUT.into()),
        tx,
    );

    debug!("starting syn scan");
    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing SYN Scan…",
    ))));

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
                        store.dispatch(Action::AddDevice(d.clone()));
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

    store.dispatch(Action::UpdateMessage(None));

    Ok(syn_results)
}

fn monitor_network(
    config: Arc<Config>,
    interface: Arc<NetworkInterface>,
    store: Arc<Store>,
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

        let rx = process_arp(
            Arc::clone(&wire.0),
            Arc::clone(&wire.1),
            &interface,
            interface.cidr.clone(),
            source_port,
            rx,
            tx.clone(),
            Arc::clone(&store),
        )?;

        let results = process_syn(
            Arc::clone(&wire.0),
            Arc::clone(&wire.1),
            &interface,
            config.ports.clone(),
            rx,
            tx.clone(),
            source_port,
            Arc::clone(&store),
        )?;

        store.dispatch(Action::UpdateAllDevices(results));

        debug!("network scan completed");

        thread::sleep(time::Duration::from_secs(15));
        let handle = monitor_network(config, interface, store);
        handle.join().unwrap()
    })
}

fn init(args: &Args, interface: &NetworkInterface) -> (Config, Arc<Store>) {
    let config_path = get_project_config_path();
    let config_manager = Arc::new(Mutex::new(ConfigManager::new(&config_path)));
    let store = Arc::new(Store::new(Arc::clone(&config_manager)));

    let manager = config_manager.lock().unwrap();
    let config: Config;
    let conf_opt = manager.get_by_cidr(&interface.cidr);
    // free up manager lock so dispatches can acquire lock as needed
    drop(manager);

    if let Some(target_config) = conf_opt {
        config = target_config;
        store.dispatch(Action::SetConfig(config.id.clone()));
    } else {
        config = Config {
            id: fakeit::animal::animal().to_lowercase(),
            cidr: interface.cidr.clone(),
            ports: args.ports.clone(),
            ..Config::default()
        };
        store.dispatch(Action::CreateAndSetConfig(config.clone()))
    }

    (config, store)
}

fn is_root() -> bool {
    match env::var("USER") {
        Ok(val) => val == "root",
        Err(_e) => false,
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    initialize_logger(&args);

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = network::get_default_interface().expect("could not get default interface");
    let (config, store) = init(&args, &interface);
    // don't do anything with handle here as this call is recursive
    // so if we join our main process will never exit
    monitor_network(Arc::new(config), Arc::new(interface), Arc::clone(&store));

    if args.debug {
        loop {}
    }

    let event_manager_channel = channel();
    let app_channel = channel();

    let event_manager = events::manager::EventManager::new(
        app_channel.0,
        event_manager_channel.1,
        Arc::clone(&store),
    );

    let application = app::create_app(event_manager_channel.0, app_channel.1, store)?;

    let handle = thread::spawn(move || event_manager.start_event_loop());

    application.launch()?;
    handle.join().unwrap()
}
