use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use mockall::Sequence;
use pnet::util::MacAddr;
use r_lanlib::{
    packet::{Reader, Sender, wire::Wire},
    scanners::{
        Device, Port, PortSet, ScanMessage, arp_scanner::ARPScanner,
        syn_scanner::SYNScanner,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};

use crate::{
    config::Config,
    ipc::{
        message::{MainMessage, NetworkMessage},
        network::NetworkIpc,
        traits::{MockIpcReceiver, MockIpcSender},
    },
    process::network::traits::NetworkMonitor,
};

use super::*;

struct StubSender;
impl Sender for StubSender {
    fn send(&mut self, _packet: &[u8]) -> r_lanlib::error::Result<()> {
        Ok(())
    }
}

struct StubReader;
impl Reader for StubReader {
    fn next_packet(&mut self) -> r_lanlib::error::Result<&[u8]> {
        Ok(&[])
    }
}

fn stub_wire() -> Wire {
    Wire(
        Arc::new(Mutex::new(StubSender)),
        Arc::new(Mutex::new(StubReader)),
    )
}

fn default_config() -> Config {
    Config::new(
        "user".to_string(),
        "/home/user/.ssh/id_rsa".to_string(),
        "192.168.1.0/24".to_string(),
    )
}

fn make_device(ip: Ipv4Addr) -> Device {
    Device {
        hostname: format!("host-{}", ip),
        ip,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    }
}

fn setup(
    mock_sender: MockIpcSender<MainMessage>,
    mock_receiver: MockIpcReceiver<NetworkMessage>,
) -> NetworkProcess {
    let ipc = NetworkIpc::new(Box::new(mock_sender), Box::new(mock_receiver));

    NetworkProcess {
        wire: stub_wire(),
        interface: Arc::new(
            r_lanlib::network::get_default_interface().unwrap(),
        ),
        ipc,
        config: RefCell::new(default_config()),
        arp_history: RefCell::new(HashMap::new()),
    }
}

fn seed_arp_history(
    process: &NetworkProcess,
    entries: Vec<(Device, MissedCount)>,
) {
    let mut history = process.arp_history.borrow_mut();
    for (device, count) in entries {
        history.insert(device.ip, (device, count));
    }
}

#[test]
fn get_latest_detected_returns_only_zero_miss_devices() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let dev1 = make_device(Ipv4Addr::new(10, 0, 0, 1));
    let dev2 = make_device(Ipv4Addr::new(10, 0, 0, 2));
    let dev3 = make_device(Ipv4Addr::new(10, 0, 0, 3));

    seed_arp_history(
        &process,
        vec![(dev1.clone(), 0), (dev2.clone(), 1), (dev3.clone(), 0)],
    );

    let result = process.get_latest_detected_arp_devices();
    assert_eq!(result.len(), 2);

    let ips: Vec<Ipv4Addr> = result.iter().map(|d| d.ip).collect();
    assert!(ips.contains(&dev1.ip));
    assert!(ips.contains(&dev3.ip));
    assert!(!ips.contains(&dev2.ip));
}

#[test]
fn get_latest_detected_returns_empty_when_all_missed() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let dev = make_device(Ipv4Addr::new(10, 0, 0, 1));
    seed_arp_history(&process, vec![(dev, 2)]);

    let result = process.get_latest_detected_arp_devices();
    assert!(result.is_empty());
}

#[test]
fn get_padded_list_includes_devices_below_max_miss() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let dev1 = make_device(Ipv4Addr::new(10, 0, 0, 1));
    let dev2 = make_device(Ipv4Addr::new(10, 0, 0, 2));
    let dev3 = make_device(Ipv4Addr::new(10, 0, 0, 3));
    let dev4 = make_device(Ipv4Addr::new(10, 0, 0, 4));

    seed_arp_history(
        &process,
        vec![
            (dev1.clone(), 0),
            // miss count 2 is still below MAX_ARP_MISS (3)
            (dev2.clone(), 2),
            // miss count 3 is equal to MAX_ARP_MISS should be included
            (dev3.clone(), 3),
            // miss count 4 is > MAX_ARP_MISS should be excluded
            (dev4.clone(), 4),
        ],
    );

    let result = process.get_padded_list_of_arp_devices();
    assert_eq!(result.len(), 3);

    let ips: Vec<Ipv4Addr> = result.iter().map(|d| d.ip).collect();
    assert!(ips.contains(&dev1.ip));
    assert!(ips.contains(&dev2.ip));
    assert!(ips.contains(&dev3.ip));
    assert!(!ips.contains(&dev4.ip));
}

#[test]
fn get_padded_list_returns_empty_when_no_history() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let result = process.get_padded_list_of_arp_devices();
    assert!(result.is_empty());
}

#[test]
fn monitor_exits_on_quit_message() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<NetworkMessage>::new();

    mock_receiver
        .expect_try_recv()
        .returning(|| Ok(NetworkMessage::Quit))
        .times(1);

    let process = setup(mock_sender, mock_receiver);

    let result = process.monitor();
    assert!(result.is_ok());
}

#[test]
fn arp_history_increments_miss_count_for_absent_devices() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let dev1 = make_device(Ipv4Addr::new(10, 0, 0, 1));
    let dev2 = make_device(Ipv4Addr::new(10, 0, 0, 2));

    seed_arp_history(&process, vec![(dev1.clone(), 0), (dev2.clone(), 0)]);

    // simulate a scan that only found dev1
    let arp_results: HashMap<Ipv4Addr, Device> =
        HashMap::from([(dev1.ip, dev1.clone())]);

    process.arp_history.borrow_mut().iter_mut().for_each(|d| {
        if !arp_results.contains_key(d.0) {
            d.1.1 += 1;
        }
    });

    let history = process.arp_history.borrow();
    assert_eq!(history.get(&dev1.ip).unwrap().1, 0);
    assert_eq!(history.get(&dev2.ip).unwrap().1, 1);
}

#[test]
fn arp_history_removes_devices_at_max_miss() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let dev1 = make_device(Ipv4Addr::new(10, 0, 0, 1));
    let dev2 = make_device(Ipv4Addr::new(10, 0, 0, 2));

    // dev2 is one miss away from removal
    seed_arp_history(
        &process,
        vec![(dev1.clone(), 0), (dev2.clone(), MAX_ARP_MISS - 1)],
    );

    // simulate scan that found neither device
    let arp_results: HashMap<Ipv4Addr, Device> = HashMap::new();

    process.arp_history.borrow_mut().iter_mut().for_each(|d| {
        if !arp_results.contains_key(d.0) {
            d.1.1 += 1;
        }
    });

    process
        .arp_history
        .borrow_mut()
        .retain(|_ip, t| t.1 < MAX_ARP_MISS);

    let history = process.arp_history.borrow();
    assert!(history.contains_key(&dev1.ip));
    assert_eq!(history.get(&dev1.ip).unwrap().1, 1);
    // dev2 reached MAX_ARP_MISS and should be removed
    assert!(!history.contains_key(&dev2.ip));
}

#[test]
fn arp_history_resets_miss_count_on_rediscovery() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();
    let process = setup(mock_sender, mock_receiver);

    let dev = make_device(Ipv4Addr::new(10, 0, 0, 1));

    // device has been missed twice
    seed_arp_history(&process, vec![(dev.clone(), 2)]);

    // re-discovered: inserting with miss count 0
    process
        .arp_history
        .borrow_mut()
        .insert(dev.ip, (dev.clone(), 0));

    let history = process.arp_history.borrow();
    assert_eq!(history.get(&dev.ip).unwrap().1, 0);
}

#[test]
fn process_arp_sends_messages_and_updates_history() {
    let mut seq = Sequence::new();
    let mut mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();

    mock_sender
        .expect_send()
        .once()
        .in_sequence(&mut seq)
        .withf(|m| matches!(m, MainMessage::ArpStart))
        .returning(|_| Ok(()));

    mock_sender
        .expect_send()
        .once()
        .in_sequence(&mut seq)
        .withf(|m| matches!(m, MainMessage::ArpUpdate(_)))
        .returning(|_| Ok(()));

    mock_sender
        .expect_send()
        .once()
        .in_sequence(&mut seq)
        .withf(|m| matches!(m, MainMessage::ArpDone))
        .returning(|_| Ok(()));

    let process = setup(mock_sender, mock_receiver);

    let device = make_device(Ipv4Addr::new(192, 168, 1, 10));

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    // pre-load messages so the loop exits before the scanner's
    // own thread produces anything
    tx.send(ScanMessage::ARPScanDevice(device.clone())).unwrap();
    tx.send(ScanMessage::Done).unwrap();

    let scanner = ARPScanner::builder()
        .interface(Arc::clone(&process.interface))
        .wire(stub_wire())
        .targets(IPTargets::new(vec!["192.168.1.0/24".to_string()]).unwrap())
        .include_host_names(false)
        .include_vendor(false)
        .idle_timeout(Duration::from_millis(50))
        .source_port(54321_u16)
        .notifier(tx)
        .build()
        .unwrap();

    let result = process.process_arp(scanner, rx);
    assert!(result.is_ok());

    let history = process.arp_history.borrow();
    assert_eq!(history.len(), 1);
    let (dev, miss_count) = history.get(&device.ip).unwrap();
    assert_eq!(dev.ip, device.ip);
    assert_eq!(*miss_count, 0);
}

#[test]
fn process_syn_sends_messages_and_returns_results() {
    let mut seq = Sequence::new();
    let mut mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<NetworkMessage>::new();

    mock_sender
        .expect_send()
        .once()
        .in_sequence(&mut seq)
        .withf(|m| matches!(m, MainMessage::SynStart))
        .returning(|_| Ok(()));

    mock_sender
        .expect_send()
        .once()
        .in_sequence(&mut seq)
        .withf(|m| matches!(m, MainMessage::SynUpdate(_)))
        .returning(|_| Ok(()));

    mock_sender
        .expect_send()
        .once()
        .in_sequence(&mut seq)
        .withf(|m| matches!(m, MainMessage::SynDone))
        .returning(|_| Ok(()));

    let process = setup(mock_sender, mock_receiver);

    let device = make_device(Ipv4Addr::new(192, 168, 1, 10));

    // seed arp_history so process_syn includes this device
    seed_arp_history(&process, vec![(device.clone(), 0)]);

    let port = Port {
        id: 80,
        service: "http".to_string(),
    };

    let mut open_ports = HashSet::new();
    open_ports.insert(port.clone());

    let device_with_port = Device {
        open_ports: open_ports.into(),
        ..device.clone()
    };

    let (tx, rx) = mpsc::channel::<ScanMessage>();

    tx.send(ScanMessage::SYNScanDevice(device_with_port))
        .unwrap();
    tx.send(ScanMessage::Done).unwrap();

    let scanner = SYNScanner::builder()
        .interface(Arc::clone(&process.interface))
        .wire(stub_wire())
        .targets(vec![device.clone()])
        .ports(PortTargets::new(vec!["80".to_string()]).unwrap())
        .idle_timeout(Duration::from_millis(50))
        .source_port(54321_u16)
        .notifier(tx)
        .build()
        .unwrap();

    let result = process.process_syn(scanner, rx);
    assert!(result.is_ok());

    let devices = result.unwrap();
    assert_eq!(devices.len(), 1);

    let found = devices.get(&device.ip).unwrap();
    assert_eq!(found.ip, device.ip);
    assert!(found.open_ports.0.contains(&port));
}
