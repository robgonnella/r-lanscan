use super::*;
use pnet::{
    packet::{arp, ethernet, ipv4, tcp},
    util,
};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{net, sync::Mutex};

use crate::{
    network,
    oui::{traits::mocks::MockOuiDb, types::OuiData},
    packet::{arp_packet::create_arp_reply, syn_packet::create_syn_reply},
    scanners::Port,
    wire::{
        PacketMetadata, Reader, Sender,
        mocks::{MockPacketReader, MockPacketSender},
    },
};

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
    let wire = Wire(sender, receiver);

    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.0/24".to_string()]).unwrap();
    let ports = PortTargets::new(vec!["2000-8000".to_string()]).unwrap();
    let (tx, _) = channel();

    let scanner = FullScanner::builder()
        .interface(interface)
        .wire(wire)
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
        ip: device_ip,
        mac: device_mac.clone(),
        vendor: "XEROX CORPORATION".to_string(),
        ..Device::default()
    };

    let mut receiver = MockPacketReader::new();
    let mut sender = MockPacketSender::new();

    #[allow(static_mut_refs)]
    receiver.expect_next_packet_with_metadata().returning(|| {
        Ok((unsafe { &ARP_PACKET }, PacketMetadata { timestamp: None }))
    });
    #[allow(static_mut_refs)]
    receiver
        .expect_next_packet()
        .returning(|| Ok(unsafe { &SYN_PACKET }));
    sender.expect_send().returning(|_| Ok(()));

    let arc_receiver: Arc<Mutex<dyn Reader>> = Arc::new(Mutex::new(receiver));
    let arc_sender: Arc<Mutex<dyn Sender>> = Arc::new(Mutex::new(sender));
    let wire = Wire(arc_sender, arc_receiver);

    let idle_timeout = Duration::from_secs(2);
    let targets = IPTargets::new(vec!["192.168.1.2".to_string()]).unwrap();
    let ports = PortTargets::new(vec!["2222".to_string()]).unwrap();
    let (tx, rx) = channel();

    let mut oui = MockOuiDb::new();

    oui.expect_lookup()
        .withf(move |mac| mac.to_string() == device_mac.to_string())
        .returning(|_| {
            Some(Box::leak(Box::new(OuiData {
                organization: "XEROX CORPORATION".to_string(),
            })))
        });

    let arc_oui: Arc<dyn Oui> = Arc::new(oui);

    let scanner = FullScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(targets)
        .ports(ports)
        .host(true)
        .vendor(true)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .source_port(54321_u16)
        .oui(arc_oui)
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
