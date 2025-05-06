use log::*;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{Reader, Sender},
    targets::{ips::IPTargets, ports::PortTargets},
};

use super::{
    arp_scanner::ARPScanner, syn_scanner::SYNScanner, Device, ScanError, ScanMessage, Scanner,
};

// Data structure representing a Full scanner (ARP + SYN)
pub struct FullScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Arc<IPTargets>,
    ports: Arc<PortTargets>,
    vendor: bool,
    host: bool,
    idle_timeout: Duration,
    notifier: mpsc::Sender<ScanMessage>,
    source_port: u16,
}

impl<'net> FullScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader: Arc<Mutex<dyn Reader>>,
        packet_sender: Arc<Mutex<dyn Sender>>,
        targets: Arc<IPTargets>,
        ports: Arc<PortTargets>,
        vendor: bool,
        host: bool,
        idle_timeout: Duration,
        notifier: mpsc::Sender<ScanMessage>,
        source_port: u16,
    ) -> Self {
        Self {
            interface,
            packet_reader,
            packet_sender,
            targets,
            ports,
            vendor,
            host,
            idle_timeout,
            notifier,
            source_port,
        }
    }
}

impl<'net> FullScanner<'net> {
    fn get_syn_targets_from_arp_scan(&self) -> Vec<Device> {
        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let mut syn_targets: Vec<Device> = Vec::new();

        let arp = ARPScanner::new(
            self.interface,
            Arc::clone(&self.packet_reader),
            Arc::clone(&self.packet_sender),
            Arc::clone(&self.targets),
            self.source_port,
            self.vendor,
            self.host,
            self.idle_timeout,
            tx.clone(),
        );

        arp.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done(_) => {
                        debug!("arp sending complete");
                        break;
                    }
                    ScanMessage::ARPScanResult(device) => {
                        syn_targets.push(device.to_owned());
                    }
                    _ => {}
                }
            }
        }

        syn_targets
    }
}

// Implements the Scanner trait for FullScanner
impl<'net> Scanner for FullScanner<'net> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
        let syn_targets = self.get_syn_targets_from_arp_scan();
        let syn = SYNScanner::new(
            self.interface,
            Arc::clone(&self.packet_reader),
            Arc::clone(&self.packet_sender),
            syn_targets,
            Arc::clone(&self.ports),
            self.source_port,
            self.idle_timeout,
            self.notifier.clone(),
        );

        syn.scan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::{
        packet::{arp, ethernet, ipv4, tcp},
        util,
    };
    use std::collections::HashSet;
    use std::net;
    use std::str::FromStr;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::network;
    use crate::packet::arp::create_arp_reply;
    use crate::packet::mocks::{MockPacketReader, MockPacketSender};
    use crate::packet::syn::create_syn_reply;
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

        let scanner = FullScanner::new(
            &interface,
            receiver,
            sender,
            targets,
            ports,
            true,
            true,
            idle_timeout,
            tx,
            54321,
        );

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
            unsafe { &mut ARP_PACKET },
        );

        let syn_packet = create_syn_reply(
            device_mac,
            device_ip,
            device_port,
            interface.mac,
            interface.ipv4,
            54321,
            unsafe { &mut SYN_PACKET },
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

        let scanner = FullScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            ports,
            true,
            true,
            idle_timeout,
            tx,
            54321,
        );

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
                    ScanMessage::Done(_) => break,
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

        let result = handle.join().unwrap().unwrap();

        assert_eq!(result, ());
        assert_eq!(detected_device.hostname, device.hostname);
        assert_eq!(detected_device.ip, device.ip);
        assert_eq!(detected_device.mac, device.mac);
        assert_eq!(detected_device.vendor, device.vendor);
        assert_eq!(detected_device.is_current_host, device.is_current_host);
        assert!(detected_device.open_ports.contains(&expected_open_port));
    }
}
