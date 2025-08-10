use super::*;
use pnet::{
    packet::{arp, ethernet, ipv4, tcp},
    util,
};
use std::collections::HashSet;
use std::net;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::network;
use crate::packet::arp_packet::create_arp_reply;
use crate::packet::mocks::{MockPacketReader, MockPacketSender};
use crate::packet::syn_packet::create_syn_reply;
use crate::scanners::{DeviceWithPorts, Port};

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
    let ports = PortTargets::new(vec!["2000-8000".to_string()]);
    let (tx, _) = channel();

    let scanner = FullScanner::new(FullScannerArgs {
        interface: &interface,
        packet_reader: receiver,
        packet_sender: sender,
        targets,
        ports,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
        source_port: 54321,
    });

    assert!(scanner.host);
    assert!(scanner.vendor);
    assert_eq!(scanner.idle_timeout, idle_timeout);
    assert_eq!(scanner.source_port, 54321);
}

#[test]
#[allow(warnings)]
fn sends_and_reads_packets() {
    static mut ARP_PACKET: [u8; PKT_TOTAL_ARP_SIZE] = [0u8; PKT_TOTAL_ARP_SIZE];
    static mut SYN_PACKET: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];

    let interface = network::get_default_interface().unwrap();
    let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
    let device_mac = util::MacAddr::default();
    let device_port = 2222;

    let arp_packet = create_arp_reply(
        device_mac,
        device_ip,
        interface.mac,
        interface.ipv4,
        #[allow(static_mut_refs)]
        unsafe {
            &mut ARP_PACKET
        },
    );

    let syn_packet = create_syn_reply(
        device_mac,
        device_ip,
        device_port,
        interface.mac,
        interface.ipv4,
        54321,
        #[allow(static_mut_refs)]
        unsafe {
            &mut SYN_PACKET
        },
    );

    let device = Device {
        hostname: "".to_string(),
        ip: device_ip.to_string(),
        mac: device_mac.to_string(),
        vendor: "XEROX CORPORATION".to_string(),
        is_current_host: false,
    };

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    let mut next_type = "arp";

    #[allow(static_mut_refs)]
    receiver.expect_next_packet().returning(move || {
        if next_type == "arp" {
            next_type = "syn";
            Ok(unsafe { &ARP_PACKET })
        } else {
            Ok(unsafe { &SYN_PACKET })
        }
    });
    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver = Arc::new(Mutex::new(receiver));
    let arc_sender = Arc::new(Mutex::new(sender));

    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
    let ports = PortTargets::new(vec!["2222".to_string()]);
    let (tx, rx) = channel();

    let scanner = FullScanner::new(FullScannerArgs {
        interface: &interface,
        packet_reader: arc_receiver,
        packet_sender: arc_sender,
        targets,
        ports,
        include_host_names: true,
        include_vendor: true,
        idle_timeout,
        notifier: tx,
        source_port: 54321,
    });

    let handle = scanner.scan();

    let mut detected_device = DeviceWithPorts {
        hostname: "".to_string(),
        ip: "".to_string(),
        is_current_host: false,
        mac: "".to_string(),
        vendor: "".to_string(),
        open_ports: HashSet::new(),
    };

    let expected_open_port = Port {
        id: device_port,
        service: "".to_string(),
    };

    loop {
        if let Ok(msg) = rx.recv() {
            match msg {
                ScanMessage::Done => break,
                ScanMessage::SYNScanResult(d) => {
                    detected_device.hostname = d.device.hostname;
                    detected_device.ip = d.device.ip;
                    detected_device.mac = d.device.mac;
                    detected_device.vendor = d.device.vendor;
                    detected_device.is_current_host = d.device.is_current_host;
                    detected_device.open_ports.insert(d.open_port);
                }
                _ => {}
            }
        }
    }

    let result = handle.join().unwrap();

    assert!(result.is_ok());
    assert_eq!(detected_device.hostname, device.hostname);
    assert_eq!(detected_device.ip, device.ip);
    assert_eq!(detected_device.mac, device.mac);
    assert_eq!(detected_device.vendor, device.vendor);
    assert_eq!(detected_device.is_current_host, device.is_current_host);
    assert!(detected_device.open_ports.contains(&expected_open_port));
}
