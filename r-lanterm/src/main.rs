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

#[cfg(test)]
mod tests {
    use mockall::mock;
    use nanoid::nanoid;
    use pnet::util::MacAddr;
    use r_lanlib::packet::{Reader, Sender};
    use r_lanlib::scanners::{Device, Port, SYNScanResult};
    use std::error::Error;
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    use super::*;

    mock! {
            pub PacketReader {}
            impl Reader for PacketReader {
                fn next_packet(&mut self) -> Result<&'static [u8], Box<dyn Error>>;
            }
    }

    mock! {
        pub PacketSender {}
        impl Sender for PacketSender {
            fn send(&mut self, packet: &[u8]) -> Result<(), Box<dyn Error>>;
        }
    }

    fn default_args(debug: bool) -> Args {
        Args {
            debug,
            ports: vec!["80".to_string()],
        }
    }

    fn setup() -> (String, NetworkInterface, Arc<Store>) {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));
        let store = Arc::new(Store::new(conf_manager));
        let interface = NetworkInterface {
            cidr: "192.168.1.1/24".to_string(),
            description: "test interface".to_string(),
            flags: 0,
            index: 0,
            ips: vec![],
            ipv4: Ipv4Addr::from_str("192.168.1.2").unwrap(),
            mac: MacAddr::default(),
            name: "test_iface".to_string(),
        };
        (tmp_path, interface, store)
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_initialize_logger() {
        let args = default_args(false);
        initialize_logger(&args);
    }

    #[test]
    fn test_get_project_config_path() {
        let p = get_project_config_path();
        assert_ne!(p, "");
    }

    #[test]
    fn test_process_arp() {
        let (conf_path, interface, store) = setup();
        let mut mock_packet_reader = MockPacketReader::new();
        let mut mock_packet_sender = MockPacketSender::new();
        let source_port = 54321;
        let (tx, rx) = channel();

        mock_packet_sender.expect_send().returning(|_| Ok(()));
        mock_packet_reader
            .expect_next_packet()
            .returning(|| Ok(&[1]));

        let device = Device {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            mac: MacAddr::default().to_string(),
            is_current_host: false,
            vendor: "vendor".to_string(),
        };

        tx.send(ScanMessage::ARPScanResult(device.clone())).unwrap();
        tx.send(ScanMessage::Done).unwrap();

        let res = process_arp(
            Arc::new(Mutex::new(mock_packet_reader)),
            Arc::new(Mutex::new(mock_packet_sender)),
            &interface,
            interface.cidr.clone(),
            source_port,
            rx,
            tx,
            Arc::clone(&store),
        );

        assert!(res.is_ok());

        let state = store.get_state();

        let expected_devices = vec![DeviceWithPorts {
            hostname: device.hostname,
            ip: device.ip,
            mac: device.mac,
            is_current_host: device.is_current_host,
            vendor: device.vendor,
            open_ports: HashSet::new(),
        }];

        assert_eq!(state.devices, expected_devices);

        tear_down(conf_path);
    }

    #[test]
    fn test_process_syn() {
        let (conf_path, interface, store) = setup();
        let mut mock_packet_reader = MockPacketReader::new();
        let mut mock_packet_sender = MockPacketSender::new();
        let source_port = 54321;
        let (tx, rx) = channel();

        mock_packet_sender.expect_send().returning(|_| Ok(()));
        mock_packet_reader
            .expect_next_packet()
            .returning(|| Ok(&[1]));

        let device = Device {
            hostname: "hostname".to_string(),
            ip: "192.168.1.1".to_string(),
            mac: MacAddr::default().to_string(),
            is_current_host: false,
            vendor: "vendor".to_string(),
        };

        let mut open_ports = HashSet::new();

        let open_port = Port {
            id: 80,
            service: "http".to_string(),
        };

        open_ports.insert(open_port.clone());

        let device_with_ports = DeviceWithPorts {
            hostname: device.hostname.clone(),
            ip: device.ip.clone(),
            mac: device.mac.clone(),
            is_current_host: device.is_current_host.clone(),
            vendor: device.vendor.clone(),
            open_ports: open_ports.clone(),
        };

        tx.send(ScanMessage::SYNScanResult(SYNScanResult {
            device: device.clone(),
            open_port: open_port.clone(),
        }))
        .unwrap();
        tx.send(ScanMessage::Done).unwrap();

        store.dispatch(Action::AddDevice(device_with_ports.clone()));

        let res = process_syn(
            Arc::new(Mutex::new(mock_packet_reader)),
            Arc::new(Mutex::new(mock_packet_sender)),
            &interface,
            vec!["80".to_string()],
            rx,
            tx,
            source_port,
            Arc::clone(&store),
        );

        assert!(res.is_ok());

        let devices = res.unwrap();

        assert_eq!(devices, vec![device_with_ports]);

        tear_down(conf_path);
    }
}
