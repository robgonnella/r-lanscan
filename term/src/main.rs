//! Terminal UI (TUI) app for managing networked LAN devices
//!
//! This is the new and improved rust version of [ops](https://github.com/robgonnella/ops)
//!
//! # Features:
//!
//! - Save global ssh configuration used for all devices
//! - Save device specific ssh config that overrides global configuration
//! - Drop to ssh for any device found on network (requires ssh client to be installed)
//! - Run `traceroute` for device found on network (requires traceroute to be installed)
//! - Open terminal web browser on any port for any device found on network (requires lynx browser to be installed)
//!
//! # Examples
//!
//! ```bash
//! # show help menu
//! sudo r-lanterm --help
//!
//! # launch application
//! sudo r-lanterm
//! ```

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use config::{Config, ConfigManager};
use core::time;
use directories::ProjectDirs;
use log::*;
use r_lanlib::{
    network::{self, NetworkInterface},
    packet::{self, Reader as WireReader, Sender as WireSender},
    scanners::{
        arp_scanner::{ARPScanner, ARPScannerArgs},
        syn_scanner::{SYNScanner, SYNScannerArgs},
        DeviceWithPorts, ScanError, ScanMessage, Scanner, IDLE_TIMEOUT,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::{
    collections::HashSet,
    env, fs,
    sync::{
        mpsc::{self, channel, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use ui::{
    app, events,
    store::{action::Action, derived::get_detected_devices, Store},
};

#[doc(hidden)]
mod config;
#[doc(hidden)]
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

#[doc(hidden)]
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

#[doc(hidden)]
fn get_project_config_path() -> String {
    let project_dir = ProjectDirs::from("", "", "r-lanterm").unwrap();
    let config_dir = project_dir.config_dir();
    fs::create_dir_all(config_dir).unwrap();
    config_dir.join("config.yml").to_str().unwrap().to_string()
}

#[doc(hidden)]
fn process_arp(
    args: ARPScannerArgs,
    rx: Receiver<ScanMessage>,
    store: Arc<Store>,
) -> Result<Receiver<ScanMessage>, ScanError> {
    let scanner = ARPScanner::new(args);

    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing ARP Scan…",
    ))));

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

#[doc(hidden)]
fn process_syn(
    args: SYNScannerArgs,
    rx: Receiver<ScanMessage>,
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

    let scanner = SYNScanner::new(args);

    debug!("starting syn scan");
    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing SYN Scan…",
    ))));

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

#[doc(hidden)]
fn monitor_network(
    exit: Receiver<()>,
    packet_reader: Arc<Mutex<dyn WireReader>>,
    packet_sender: Arc<Mutex<dyn WireSender>>,
    config: Arc<Config>,
    interface: Arc<NetworkInterface>,
    store: Arc<Store>,
) -> JoinHandle<Result<(), ScanError>> {
    info!("starting network monitor");

    thread::spawn(move || -> Result<(), ScanError> {
        loop {
            let res = exit.try_recv();

            if res.is_ok() {
                return Ok(());
            }

            let source_port = network::get_available_port().map_err(|e| ScanError {
                ip: None,
                port: None,
                error: Box::from(e),
            })?;

            let (tx, rx) = mpsc::channel::<ScanMessage>();

            let rx = process_arp(
                ARPScannerArgs {
                    interface: &interface,
                    packet_reader: Arc::clone(&packet_reader),
                    packet_sender: Arc::clone(&packet_sender),
                    targets: IPTargets::new(vec![interface.cidr.clone()]),
                    include_host_names: true,
                    include_vendor: true,
                    idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
                    source_port,
                    notifier: tx.clone(),
                },
                rx,
                Arc::clone(&store),
            )?;

            let state = store.get_state();
            let arp_devices = get_detected_devices(&state);

            let results = process_syn(
                SYNScannerArgs {
                    interface: &interface,
                    packet_reader: Arc::clone(&packet_reader),
                    packet_sender: Arc::clone(&packet_sender),
                    targets: arp_devices,
                    ports: PortTargets::new(config.ports.clone()),
                    source_port,
                    idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
                    notifier: tx.clone(),
                },
                rx,
                Arc::clone(&store),
            )?;

            store.dispatch(Action::UpdateAllDevices(results));

            debug!("network scan completed");

            thread::sleep(time::Duration::from_secs(15));
        }
    })
}

#[doc(hidden)]
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

#[doc(hidden)]
fn is_root() -> bool {
    match env::var("USER") {
        Ok(val) => val == "root",
        Err(_e) => false,
    }
}

#[doc(hidden)]
fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    initialize_logger(&args);

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = network::get_default_interface().expect("could not get default interface");
    let (config, store) = init(&args, &interface);
    let (_, exit_rx) = mpsc::channel();

    let wire = packet::wire::default(&interface).map_err(|e| ScanError {
        ip: None,
        port: None,
        error: e,
    })?;

    monitor_network(
        exit_rx,
        wire.0,
        wire.1,
        Arc::new(config),
        Arc::new(interface),
        Arc::clone(&store),
    );

    if args.debug {
        let mut signals = Signals::new([SIGINT]).unwrap();
        let _ = signals.wait();
        return Ok(());
    }

    // captures ctrl-c only in main thread so when we drop down to shell
    // commands like ssh, we will pause the key handler for ctrl-c in app
    // and capture ctrl-c here to prevent exiting app and just let ctrl-c
    // be handled by the command being executed, which should return us
    // to our app where we can restart our ui and key-handlers
    ctrlc::set_handler(move || println!("captured ctrl-c!")).expect("Error setting Ctrl-C handler");

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

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
