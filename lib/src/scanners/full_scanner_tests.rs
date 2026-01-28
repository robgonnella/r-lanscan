use super::*;
use pnet::{
    packet::{arp, ethernet, ipv4, tcp},
    util,
};
use std::net;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::packet::arp_packet::create_arp_reply;
use crate::packet::mocks::{MockPacketReader, MockPacketSender};
use crate::packet::syn_packet::create_syn_reply;
use crate::scanners::Port;
use crate::{network, scanners::PortSet};

const PKT_ETH_SIZE: usize = ethernet::EthernetPacket::minimum_packet_size();
const PKT_ARP_SIZE: usize = arp::ArpPacket::minimum_packet_size();
const PKT_TOTAL_ARP_SIZE: usize = PKT_ETH_SIZE + PKT_ARP_SIZE;

const PKT_IP4_SIZE: usize = ipv4::Ipv4Packet::minimum_packet_size();
const PKT_TCP_SIZE: usize = tcp::TcpPacket::minimum_packet_size();
const PKT_TOTAL_SYN_SIZE: usize = PKT_ETH_SIZE + PKT_IP4_SIZE + PKT_TCP_SIZE;

#[test]
fn new() {
    let interface = Arc::new(network::get_default_interface().unwrap());
    let sender: Arc<Mutex<dyn Sender>> =
        Arc::new(Mutex::new(MockPacketSender::new()));
    let receiver: Arc<Mutex<dyn Reader>> =
        Arc::new(Mutex::new(MockPacketReader::new()));
    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.0/24".to_string()]).unwrap();
    let ports = PortTargets::new(vec!["2000-8000".to_string()]).unwrap();
    let (tx, _) = channel();

    let scanner = FullScanner::builder()
        .interface(interface)
        .packet_reader(receiver)
        .packet_sender(sender)
        .targets(targets)
        .ports(ports)
        .host(true)
        .vendor(true)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .source_port(54321_u16)
        .build()
        .unwrap();

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

    let interface = Arc::new(network::get_default_interface().unwrap());
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
        ip: device_ip,
        mac: device_mac,
        vendor: "XEROX CORPORATION".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
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

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));

    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]).unwrap();
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();
    let (tx, rx) = channel();

    let scanner = FullScanner::builder()
        .interface(interface)
        .packet_reader(arc_receiver)
        .packet_sender(arc_sender)
        .targets(targets)
        .ports(ports)
        .host(true)
        .vendor(true)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .source_port(54321_u16)
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
                ScanMessage::Done => break,
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
