use super::*;
use pnet::{
    packet::{arp, ethernet, ipv4, tcp},
    util,
};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::network;
use crate::packet::arp_packet::create_arp_reply;
use crate::packet::mocks::{MockPacketReader, MockPacketSender};
use crate::packet::syn_packet::create_syn_reply;

const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = arp::ArpPacket::minimum_packet_size();
const PKT_TOTAL_ARP_SIZE: usize = PKT_ETH_SIZE + PKT_ARP_SIZE;

const PKT_IP4_SIZE: usize = ipv4::Ipv4Packet::minimum_packet_size();
const PKT_TCP_SIZE: usize = tcp::TcpPacket::minimum_packet_size();
const PKT_TOTAL_SYN_SIZE: usize = PKT_ETH_SIZE + PKT_IP4_SIZE + PKT_TCP_SIZE;

#[test]
fn new() {
    let interface = network::get_default_interface().unwrap();
    let sender = Arc::new(Mutex::new(MockPacketSender::new()));
    let receiver = Arc::new(Mutex::new(MockPacketReader::new()));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.0/24".to_string()]);
    let (tx, _) = channel();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: receiver,
        packet_sender: sender,
        targets,
        source_port: 54321,
        include_vendor: true,
        include_host_names: true,
        idle_timeout,
        notifier: tx,
    });

    assert!(scanner.include_host_names);
    assert!(scanner.include_vendor);
    assert_eq!(scanner.idle_timeout, idle_timeout);
    assert_eq!(scanner.source_port, 54321);
}

#[test]
#[allow(warnings)]
fn sends_and_reads_packets() {
    static mut PACKET: [u8; PKT_TOTAL_ARP_SIZE] = [0u8; PKT_TOTAL_ARP_SIZE];
    let interface = network::get_default_interface().unwrap();
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();

    create_arp_reply(
        device_mac,
        device_ip,
        interface.mac,
        interface.ipv4,
        #[allow(static_mut_refs)]
        unsafe {
            &mut PACKET
        },
    );

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .returning(|| Ok(unsafe { &PACKET }));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec![device_ip.to_string()]);
    let (tx, rx) = channel();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let handle = scanner.scan();

    let mut detected_device = Device {
        hostname: "".to_string(),
        ip: "".to_string(),
        is_current_host: false,
        mac: "".to_string(),
        vendor: "".to_string(),
    };

    loop {
        if let Ok(msg) = rx.recv() {
            match msg {
                ScanMessage::Done => {
                    break;
                }
                ScanMessage::ARPScanResult(device) => {
                    detected_device = device;
                }
                _ => {}
            }
        }
    }

    let result = handle.join().unwrap();
    assert!(result.is_ok());
    assert_eq!(detected_device.mac.to_string(), device_mac.to_string());
    assert_eq!(detected_device.ip.to_string(), device_ip.to_string());
}

#[test]
#[allow(warnings)]
fn ignores_unrelated_packets() {
    static mut PACKET: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];
    let interface = network::get_default_interface().unwrap();
    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    for i in 0..5 {
        let mac = util::MacAddr::default();
        let ip = net::Ipv4Addr::from_str(format!("192.168.0.{}", 1 + i).as_str()).unwrap();

        create_syn_reply(
            mac,
            ip,
            8080,
            interface.mac,
            interface.ipv4,
            54321,
            #[allow(static_mut_refs)]
            unsafe {
                &mut PACKET
            },
        );

        #[allow(static_mut_refs)]
        receiver
            .expect_next_packet()
            .returning(|| Ok(unsafe { &PACKET }));
    }

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);

    let (tx, rx) = channel();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

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
                ScanMessage::ARPScanResult(device) => {
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
    let interface = network::get_default_interface().unwrap();

    let receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_receiver_clone = Arc::clone(&arc_receiver);
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let (tx, _rx) = channel();

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = arc_receiver_clone.lock().unwrap(); // Acquire the lock
        panic!("Simulated panic"); // Simulate a panic
    });

    let _ = handle.join();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let (_done_tx, done_rx) = channel();

    let handle = scanner.read_packets(done_rx);

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_packet_read_error() {
    let interface = network::get_default_interface().unwrap();
    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver
        .expect_next_packet()
        .returning(|| Err(RLanLibError::Wire("oh no an error".into())));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let (tx, _rx) = channel();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let (_done_tx, done_rx) = channel();

    let handle = scanner.read_packets(done_rx);

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_notifier_send_errors() {
    let interface = network::get_default_interface().unwrap();
    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver.expect_next_packet().returning(|| Ok(&[1]));
    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let (tx, rx) = channel();

    // this will cause an error when scanner tries to notify
    drop(rx);

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let handle = scanner.scan();

    let result = handle.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn reports_error_on_packet_sender_lock_errors() {
    let interface = network::get_default_interface().unwrap();
    let mut receiver = MockPacketReader::new();
    let sender = MockPacketSender::new();

    receiver.expect_next_packet().returning(|| Ok(&[1]));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let arc_sender_clone = Arc::clone(&arc_sender);
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let (tx, rx) = channel();

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = arc_sender_clone.lock().unwrap(); // Acquire the lock
        panic!("Simulated panic"); // Simulate a panic
    });

    let _ = handle.join();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let handle = scanner.scan();

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
    let interface = network::get_default_interface().unwrap();
    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver.expect_next_packet().returning(|| Ok(&[1]));
    sender
        .expect_send()
        .returning(|_| Err(RLanLibError::Wire("oh no a send error".into())));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let (tx, rx) = channel();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let handle = scanner.scan();

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
    let interface = network::get_default_interface().unwrap();
    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    receiver
        .expect_next_packet()
        .returning(|| Err(RLanLibError::Wire("oh no a read error".into())));

    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let (tx, rx) = channel();

    let scanner = ARPScanner::new(ARPScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        source_port: 54321,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
    });

    let handle = scanner.scan();

    loop {
        if let Ok(ScanMessage::Done) = rx.recv() {
            break;
        }
    }

    let result = handle.join().unwrap();

    assert!(result.is_err());
}
