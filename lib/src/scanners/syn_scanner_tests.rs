use super::*;
use core::net;
use pnet::packet::{arp, ethernet, ipv4, tcp};
use pnet::util;
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::network;
use crate::packet::arp_packet::create_arp_reply;
use crate::packet::mocks::{MockPacketReader, MockPacketSender};
use crate::packet::syn_packet::create_syn_reply;
use crate::packet::{Reader, Sender};

const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = arp::ArpPacket::minimum_packet_size();
const PKT_TOTAL_ARP_SIZE: usize = PKT_ETH_SIZE + PKT_ARP_SIZE;

const PKT_IP4_SIZE: usize = ipv4::Ipv4Packet::minimum_packet_size();
const PKT_TCP_SIZE: usize = tcp::TcpPacket::minimum_packet_size();
const PKT_TOTAL_SYN_SIZE: usize = PKT_ETH_SIZE + PKT_IP4_SIZE + PKT_TCP_SIZE;

#[test]
fn new() {
    let interface = Arc::new(network::get_default_interface().unwrap());

    let arc_receiver: Arc<Mutex<dyn Reader>> =
        Arc::new(Mutex::new(MockPacketReader::new()));
    let arc_sender: Arc<Mutex<dyn Sender>> =
        Arc::new(Mutex::new(MockPacketSender::new()));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let devices: Vec<Device> = Vec::new();
    let ports = PortTargets::new(vec!["2000-8000".to_string()]).unwrap();
    let (tx, _) = channel();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices.clone())
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    assert_eq!(scanner.targets, devices);
    assert_eq!(scanner.idle_timeout, idle_timeout);
    assert_eq!(scanner.source_port, 54321);
}

#[test]
#[allow(warnings)]
fn sends_and_reads_packets() {
    static mut PACKET: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];

    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();
    let device_port = 2222;

    create_syn_reply(
        device_mac,
        device_ip,
        device_port,
        interface.mac,
        interface.ipv4,
        54321,
        #[allow(static_mut_refs)]
        unsafe {
            &mut PACKET
        },
    );

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .returning(|| Ok(unsafe { &PACKET }));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();
    let (tx, rx) = channel();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let handle = scanner.scan().unwrap();

    let mut detected_device = None;

    let expected_open_port = Port {
        id: device_port,
        service: "".to_string(),
    };

    loop {
        if let Ok(msg) = rx.recv() {
            match msg {
                ScanMessage::Done => {
                    break;
                }
                ScanMessage::SYNScanDevice(device) => {
                    detected_device = Some(device);
                }
                _ => {}
            }
        }
    }

    let result = handle.join().unwrap();
    let detected_device = detected_device.unwrap();

    assert!(result.is_ok());
    assert_eq!(detected_device.hostname, device.hostname);
    assert_eq!(detected_device.ip, device.ip);
    assert_eq!(detected_device.mac, device.mac);
    assert_eq!(detected_device.vendor, device.vendor);
    assert_eq!(detected_device.is_current_host, device.is_current_host);
    assert!(detected_device.open_ports.0.contains(&expected_open_port));
}

#[test]
#[allow(warnings)]
fn ignores_unrelated_packets() {
    static mut SYN_PACKET1: [u8; PKT_TOTAL_SYN_SIZE] =
        [0u8; PKT_TOTAL_SYN_SIZE];
    static mut SYN_PACKET2: [u8; PKT_TOTAL_SYN_SIZE] =
        [0u8; PKT_TOTAL_SYN_SIZE];
    static mut ARP_PACKET: [u8; PKT_TOTAL_ARP_SIZE] = [0u8; PKT_TOTAL_ARP_SIZE];

    let interface = Arc::new(network::get_default_interface().unwrap());
    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();
    let device_ip = net::Ipv4Addr::from_str("192.168.0.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    // incorrect destination port
    create_syn_reply(
        device_mac.clone(),
        device_ip.clone(),
        2222,
        interface.mac,
        interface.ipv4,
        54322,
        #[allow(static_mut_refs)]
        unsafe {
            &mut SYN_PACKET1
        },
    );

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .returning(|| Ok(unsafe { &SYN_PACKET1 }));

    // incorrect address
    create_syn_reply(
        device_mac.clone(),
        net::Ipv4Addr::from_str("192.168.2.2").unwrap(),
        2222,
        interface.mac,
        interface.ipv4,
        54321,
        #[allow(static_mut_refs)]
        unsafe {
            &mut SYN_PACKET2
        },
    );

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .returning(|| Ok(unsafe { &SYN_PACKET2 }));

    // ignores arp packet
    // incorrect address
    create_arp_reply(
        device_mac.clone(),
        device_ip.clone(),
        interface.mac,
        interface.ipv4,
        #[allow(static_mut_refs)]
        unsafe {
            &mut ARP_PACKET
        },
    );

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .returning(|| Ok(unsafe { &ARP_PACKET }));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);

    let (tx, rx) = channel();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let (done_tx, done_rx) = channel();

    scanner.read_packets(done_rx);

    let mut detected_devices: Vec<Device> = Vec::new();

    let mut count = 0;
    loop {
        if count >= 8 {
            done_tx.send(()).unwrap();
            break;
        }

        if let Ok(msg) = rx.try_recv() {
            match msg {
                ScanMessage::Done => {
                    break;
                }
                ScanMessage::SYNScanDevice(device) => {
                    detected_devices.push(device);
                }
                _ => {}
            }
        } else {
            count += 1;
            thread::sleep(Duration::from_secs(1));
        }
    }

    assert_eq!(detected_devices.len(), 0);
}

#[test]
fn reports_error_on_packet_reader_lock() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let arc_receiver_clone = Arc::clone(&arc_receiver);
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, _rx) = channel();

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = arc_receiver_clone.lock().unwrap(); // Acquire the lock
        panic!("Simulated panic"); // Simulate a panic
    });

    let _ = handle.join();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let (_done_tx, done_rx) = channel();

    let handle = scanner.read_packets(done_rx).unwrap();

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
#[allow(warnings)]
fn reports_error_on_rst_packet_sender_lock() {
    static mut PACKET: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    create_syn_reply(
        device_mac.clone(),
        device_ip.clone(),
        2222,
        interface.mac,
        interface.ipv4,
        54321,
        #[allow(static_mut_refs)]
        unsafe {
            &mut PACKET
        },
    );

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .return_once(|| Ok(unsafe { &PACKET }));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let arc_sender_clone = Arc::clone(&arc_sender);
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, _rx) = channel();

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = arc_sender_clone.lock().unwrap(); // Acquire the lock
        panic!("Simulated panic"); // Simulate a panic
    });

    let _ = handle.join();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let (_done_tx, done_rx) = channel();

    let handle = scanner.read_packets(done_rx).unwrap();

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
#[allow(warnings)]
fn reports_error_on_rst_packet_send_errors() {
    static mut PACKET: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    create_syn_reply(
        device_mac.clone(),
        device_ip.clone(),
        2222,
        interface.mac,
        interface.ipv4,
        54321,
        #[allow(static_mut_refs)]
        unsafe {
            &mut PACKET
        },
    );

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .return_once(|| Ok(unsafe { &PACKET }));

    sender.expect_send().returning(|_| {
        Err(RLanLibError::Wire("oh no packet send error".into()))
    });

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let arc_sender_clone = Arc::clone(&arc_sender);
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, _rx) = channel();

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = arc_sender_clone.lock().unwrap(); // Acquire the lock
        panic!("Simulated panic"); // Simulate a panic
    });

    let _ = handle.join();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let (_done_tx, done_rx) = channel();

    let handle = scanner.read_packets(done_rx).unwrap();

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_packet_read_error() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver
        .expect_next_packet()
        .returning(|| Err(RLanLibError::Wire("oh no an error".into())));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, _rx) = channel();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let (_done_tx, done_rx) = channel();

    let handle = scanner.read_packets(done_rx).unwrap();

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_notifier_send_errors() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver.expect_next_packet().returning(|| Ok(&[1]));
    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, rx) = channel();

    // this will cause an error when scanner tries to notify
    drop(rx);

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let handle = scanner.scan().unwrap();

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_packet_sender_lock_errors() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let receiver = MockPacketReader::new();
    let sender = MockPacketSender::new();

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let arc_sender_clone = Arc::clone(&arc_sender);
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, rx) = channel();

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = arc_sender_clone.lock().unwrap(); // Acquire the lock
        panic!("Simulated panic"); // Simulate a panic
    });

    let _ = handle.join();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let handle = scanner.scan().unwrap();

    loop {
        if let Ok(ScanMessage::Done) = rx.recv() {
            break;
        }
    }

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_packet_send_errors() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver.expect_next_packet().returning(|| Ok(&[1]));
    sender
        .expect_send()
        .returning(|_| Err(RLanLibError::Wire("oh no a send error".into())));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, rx) = channel();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let handle = scanner.scan().unwrap();

    loop {
        if let Ok(ScanMessage::Done) = rx.recv() {
            break;
        }
    }

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_errors_from_read_handle() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    let device = Device {
        ip: device_ip,
        mac: device_mac,
        ..Device::default()
    };

    let devices: Vec<Device> = vec![device.clone()];
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver
        .expect_next_packet()
        .returning(|| Err(RLanLibError::Wire("oh no a read error".into())));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let (tx, rx) = channel();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(ports)
        .source_port(54321_u16)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let handle = scanner.scan().unwrap();

    loop {
        if let Ok(ScanMessage::Done) = rx.recv() {
            break;
        }
    }

    let result = handle.join().unwrap();

    assert!(result.is_err());
}
