use mockall::mock;
use nanoid::nanoid;
use pnet::util::MacAddr;
use r_lanlib::packet::{Reader, Sender};
use r_lanlib::scanners::{Device, Port, SYNScanResult};
use std::error::Error;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

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

fn mock_interface() -> NetworkInterface {
    NetworkInterface {
        cidr: "192.168.1.1/24".to_string(),
        description: "test interface".to_string(),
        flags: 0,
        index: 0,
        ips: vec![],
        ipv4: Ipv4Addr::from_str("192.168.1.2").unwrap(),
        mac: MacAddr::default(),
        name: "test_interface".to_string(),
    }
}

fn setup() -> (String, NetworkInterface, Arc<Store>) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));
    let store = Arc::new(Store::new(conf_manager));
    let interface = mock_interface();
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
        ARPScannerArgs {
            interface: &interface,
            packet_reader: Arc::new(Mutex::new(mock_packet_reader)),
            packet_sender: Arc::new(Mutex::new(mock_packet_sender)),
            targets: IPTargets::new(vec![interface.cidr.clone()]),
            include_host_names: true,
            include_vendor: true,
            source_port,
            idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
            notifier: tx,
        },
        rx,
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
        is_current_host: device.is_current_host,
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
        SYNScannerArgs {
            interface: &interface,
            packet_reader: Arc::new(Mutex::new(mock_packet_reader)),
            packet_sender: Arc::new(Mutex::new(mock_packet_sender)),
            targets: vec![device],
            ports: PortTargets::new(vec!["80".to_string()]),
            source_port,
            idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
            notifier: tx,
        },
        rx,
        Arc::clone(&store),
    );

    assert!(res.is_ok());

    let devices = res.unwrap();

    assert_eq!(devices, vec![device_with_ports]);

    tear_down(conf_path);
}

#[test]
fn test_monitor_network() {
    let (conf_path, interface, store) = setup();
    let mut mock_packet_reader = MockPacketReader::new();
    let mut mock_packet_sender = MockPacketSender::new();
    let config = Config::default();
    let (exit_tx, exit_rx) = channel();

    mock_packet_sender.expect_send().returning(|_| Ok(()));
    mock_packet_reader
        .expect_next_packet()
        .returning(|| Ok(&[1]));

    let _ = thread::spawn(move || {
        thread::sleep(Duration::from_millis(1000));
        exit_tx.send(())
    })
    .join()
    .unwrap();

    let handle = monitor_network(
        exit_rx,
        Arc::new(Mutex::new(mock_packet_reader)),
        Arc::new(Mutex::new(mock_packet_sender)),
        Arc::new(config),
        Arc::new(interface),
        store,
    );

    let _ = handle.join().unwrap();

    tear_down(conf_path);
}

#[test]
fn test_init() {
    let args = default_args(false);
    let interface = mock_interface();
    let (_config, _store) = init(&args, &interface);
}
