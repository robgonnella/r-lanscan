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
use color_eyre::eyre::{ContextCompat, Report, Result, eyre};
use config::{Config, ConfigManager};
use core::time;
use directories::ProjectDirs;
use log::*;
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::{
    any::Any,
    collections::HashMap,
    fs,
    net::Ipv4Addr,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, channel},
    },
    thread,
};

use r_lanlib::{
    network::{self, NetworkInterface},
    packet::{self, Reader as WireReader, Sender as WireSender},
    scanners::{
        Device, IDLE_TIMEOUT, PortSet, ScanMessage, Scanner,
        arp_scanner::{ARPScanner, ARPScannerArgs},
        syn_scanner::{SYNScanner, SYNScannerArgs},
    },
    targets::{ips::IPTargets, ports::PortTargets},
};

use ui::store::{Store, action::Action, derived::get_detected_arp_devices};

use crate::{
    config::DEFAULT_CONFIG_ID,
    shell::command::Command,
    ui::{colors::Theme, store::Dispatcher},
};

#[doc(hidden)]
mod config;
#[doc(hidden)]
mod ipc;
#[doc(hidden)]
mod renderer;
#[doc(hidden)]
mod shell;
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
fn initialize_logger(args: &Args) -> Result<()> {
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
    )?;

    Ok(())
}

#[doc(hidden)]
fn get_project_config_path() -> Result<String> {
    let project_dir =
        ProjectDirs::from("", "", "r-lanterm").ok_or(eyre!("failed to get project directory"))?;
    let config_dir = project_dir.config_dir();
    fs::create_dir_all(config_dir)?;
    let config_file_path = config_dir
        .join("config.yml")
        .to_str()
        .ok_or(eyre!("unable to construct config file path"))?
        .to_string();
    Ok(config_file_path)
}

#[doc(hidden)]
fn report_from_thread_panic(e: Box<dyn Any + Send>) -> Report {
    if let Some(value) = e.downcast_ref::<&str>() {
        eyre!("thread panicked with {value}")
    } else if let Some(value) = e.downcast_ref::<&String>() {
        eyre!("thread panicked with {value}")
    } else {
        eyre!("thread panicked for unknown reason")
    }
}

#[doc(hidden)]
fn process_arp(
    args: ARPScannerArgs,
    rx: Receiver<ScanMessage>,
    dispatcher: Arc<dyn Dispatcher>,
) -> Result<Receiver<ScanMessage>> {
    let scanner = ARPScanner::new(args);

    dispatcher.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing ARP Scan…",
    ))));

    let handle = scanner.scan();

    loop {
        let msg = rx.recv()?;

        match msg {
            ScanMessage::Done => {
                debug!("scanning complete");
                break;
            }
            ScanMessage::ARPScanDevice(d) => {
                debug!("received scanning message: {:?}", d);
                dispatcher.dispatch(Action::AddDevice(d));
            }
            _ => {}
        }
    }

    debug!("waiting for arp handle to finish");

    handle.join().map_err(report_from_thread_panic)??;

    dispatcher.dispatch(Action::UpdateMessage(None));

    debug!("finished arp scan");
    Ok(rx)
}

#[doc(hidden)]
fn process_syn(
    args: SYNScannerArgs,
    rx: Receiver<ScanMessage>,
    store: Arc<Store>,
) -> Result<HashMap<Ipv4Addr, Device>> {
    let state = store.get_state()?;
    let arp_devices = get_detected_arp_devices(&state);
    let mut syn_results: HashMap<Ipv4Addr, Device> = HashMap::new();

    for d in arp_devices.iter() {
        syn_results.insert(
            d.ip,
            Device {
                hostname: d.hostname.to_owned(),
                ip: d.ip.to_owned(),
                mac: d.mac.to_owned(),
                vendor: d.vendor.to_owned(),
                is_current_host: d.is_current_host,
                open_ports: PortSet::new(),
            },
        );
    }

    let scanner = SYNScanner::new(args);

    debug!("starting syn scan");
    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing SYN Scan…",
    ))));

    let handle = scanner.scan();

    loop {
        let msg = rx.recv()?;

        match msg {
            ScanMessage::Done => {
                debug!("scanning complete");
                break;
            }
            ScanMessage::SYNScanDevice(device) => {
                debug!("received syn scanning device: {:?}", device);
                let result = syn_results.get_mut(&device.ip);
                match result {
                    Some(d) => {
                        d.open_ports.0.extend(device.open_ports.0);
                        store.dispatch(Action::AddDevice(d.clone()));
                    }
                    None => {
                        warn!("received syn result for unknown device: {:?}", device);
                    }
                }
            }
            _ => {}
        }
    }

    handle.join().map_err(report_from_thread_panic)??;

    store.dispatch(Action::UpdateMessage(None));

    Ok(syn_results)
}

#[doc(hidden)]
fn monitor_network(
    exit: Receiver<()>,
    packet_reader: Arc<Mutex<dyn WireReader>>,
    packet_sender: Arc<Mutex<dyn WireSender>>,
    config: Config,
    interface: Arc<NetworkInterface>,
    store: Arc<Store>,
) -> Result<()> {
    info!("starting network monitor");

    loop {
        let res = exit.try_recv();

        if res.is_ok() {
            return Ok(());
        }

        let source_port = network::get_available_port()?;

        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let rx = process_arp(
            ARPScannerArgs {
                interface: &interface,
                packet_reader: Arc::clone(&packet_reader),
                packet_sender: Arc::clone(&packet_sender),
                targets: IPTargets::new(vec![interface.cidr.clone()])
                    .map_err(|e| eyre!("Invalid IP targets: {}", e))?,
                include_host_names: true,
                include_vendor: true,
                idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
                source_port,
                notifier: tx.clone(),
            },
            rx,
            Arc::clone(&store) as Arc<dyn Dispatcher>,
        )?;

        let state = store.get_state()?;
        let arp_devices = get_detected_arp_devices(&state);

        let results = process_syn(
            SYNScannerArgs {
                interface: &interface,
                packet_reader: Arc::clone(&packet_reader),
                packet_sender: Arc::clone(&packet_sender),
                targets: arp_devices,
                ports: PortTargets::new(config.ports.clone())
                    .map_err(|e| eyre!("Invalid port targets: {}", e))?,
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
}

#[doc(hidden)]
fn init(args: &Args, interface: &NetworkInterface) -> Result<(Config, Arc<Store>)> {
    let user = whoami::username()?;
    let home = dirs::home_dir().wrap_err(eyre!("failed to get user's home directory"))?;
    let identity = format!("{}/.ssh/id_rsa", home.to_string_lossy());

    let config_path = get_project_config_path()?;

    let config_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(interface.cidr.clone())
        .path(config_path)
        .build()?;

    let config_manager = Arc::new(Mutex::new(config_manager));

    let manager = config_manager
        .lock()
        .map_err(|e| eyre!("failed to get lock on config_manager: {}", e))?;

    let mut create_config = false;

    let current_config = manager
        .get_by_cidr(&interface.cidr)
        .or_else(|| manager.get_by_id(DEFAULT_CONFIG_ID))
        .unwrap_or_else(|| {
            create_config = true;
            Config {
                id: fakeit::animal::animal().to_lowercase(),
                cidr: interface.cidr.clone(),
                ports: args.ports.clone(),
                ..Config::new(user.clone(), identity.clone(), interface.cidr.clone())
            }
        });

    let current_config_id = current_config.id.clone();

    // free up manager lock so dispatches can acquire lock as needed
    drop(manager);

    let store = Arc::new(Store::new(
        Arc::clone(&config_manager),
        current_config.clone(),
    ));

    if create_config {
        store.dispatch(Action::CreateAndSetConfig(current_config.clone()));
    } else {
        store.dispatch(Action::SetConfig(current_config_id));
    }

    Ok((current_config, store))
}

#[doc(hidden)]
fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

#[doc(hidden)]
fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    initialize_logger(&args)?;

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = network::get_default_interface()
        .ok_or_else(|| eyre!("Could not detect default network interface"))?;
    let (config, store) = init(&args, &interface)?;
    let theme = Theme::from_string(&config.theme);
    let (_, exit_rx) = mpsc::channel();

    let wire = packet::wire::default(&interface)?;

    let net_monitor_store = Arc::clone(&store);
    // ignore handle here - we will forcefully exit instead of waiting
    // for scan to finish
    thread::spawn(move || -> Result<()> {
        monitor_network(
            exit_rx,
            wire.0,
            wire.1,
            config,
            Arc::new(interface),
            net_monitor_store,
        )
    });

    if args.debug {
        let mut signals = Signals::new([SIGINT])?;
        let _ = signals.wait();
        return Ok(());
    }

    // captures ctrl-c only in main thread so when we drop down to shell
    // commands like ssh, we will pause the key handler for ctrl-c in app
    // and capture ctrl-c here to prevent exiting app and just let ctrl-c
    // be handled by the command being executed, which should return us
    // to our app where we can restart our ui and key-handlers
    ctrlc::set_handler(move || println!("captured ctrl-c!"))?;

    let event_manager_channel = channel();
    let app_channel = channel();

    let executor = Command::new();
    let event_manager = ipc::manager::IpcManager::new(
        app_channel.0,
        event_manager_channel.1,
        Arc::clone(&store),
        Box::new(executor),
    );

    let cross_term_renderer = renderer::cross_term::create_renderer(
        event_manager_channel.0,
        app_channel.1,
        theme,
        store,
    )?;

    let event_handle = thread::spawn(move || event_manager.start_event_loop());

    cross_term_renderer.launch()?;
    event_handle.join().map_err(report_from_thread_panic)??;
    Ok(())
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
