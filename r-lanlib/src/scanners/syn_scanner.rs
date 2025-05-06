use log::*;
use pnet::{
    packet::{ethernet, ip, ipv4, tcp, Packet},
    util,
};
use std::{
    io::{Error as IOError, ErrorKind},
    net,
    str::FromStr,
    sync::{self, mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, rst::RSTPacket, syn::SYNPacket, Reader, Sender},
    scanners::{heartbeat::HeartBeat, ScanError, Scanning},
    targets::ports::PortTargets,
};

use super::{Device, Port, SYNScanResult, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct SYNScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Vec<Device>,
    ports: Arc<PortTargets>,
    source_port: u16,
    idle_timeout: Duration,
    notifier: mpsc::Sender<ScanMessage>,
}

impl<'net> SYNScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader: Arc<Mutex<dyn Reader>>,
        packet_sender: Arc<Mutex<dyn Sender>>,
        targets: Vec<Device>,
        ports: Arc<PortTargets>,
        source_port: u16,
        idle_timeout: Duration,
        notifier: mpsc::Sender<ScanMessage>,
    ) -> Self {
        Self {
            interface,
            packet_reader,
            packet_sender,
            targets,
            ports,
            source_port,
            idle_timeout,
            notifier,
        }
    }
}

impl<'net> SYNScanner<'net> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done_rx: mpsc::Receiver<()>) -> JoinHandle<Result<(), ScanError>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let heartbeat_packet_sender = Arc::clone(&self.packet_sender);
        let rst_packet_sender = Arc::clone(&self.packet_sender);
        let devices = self.targets.to_owned();
        let notifier = self.notifier.clone();
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let source_port = self.source_port.to_owned();
        let (heartbeat_tx, heartbeat_rx) = sync::mpsc::channel::<()>();

        // since reading packets off the wire is a blocking operation, we
        // won't be able to detect a "done" signal if no packets are being
        // received as we'll be blocked on waiting for one to come it. To fix
        // this we send periodic "heartbeat" packets so we can continue to
        // check for "done" signals
        thread::spawn(move || {
            debug!("starting syn heartbeat thread");
            let heartbeat = HeartBeat::new(
                source_mac,
                source_ipv4,
                source_port,
                heartbeat_packet_sender,
            );
            let interval = Duration::from_secs(1);
            loop {
                if let Ok(_) = heartbeat_rx.try_recv() {
                    debug!("stopping syn heartbeat");
                    break;
                }
                debug!("sending syn heartbeat");
                heartbeat.beat();
                thread::sleep(interval);
            }
        });

        thread::spawn(move || -> Result<(), ScanError> {
            let mut reader = packet_reader.lock().or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(e.to_string()),
                })
            })?;

            loop {
                if let Ok(_) = done_rx.try_recv() {
                    debug!("exiting syn packet reader");
                    if let Err(e) = heartbeat_tx.send(()) {
                        error!("failed to stop heartbeat: {}", e.to_string());
                    }

                    break;
                }

                let pkt = reader.next_packet().or_else(|e| {
                    Err(ScanError {
                        ip: None,
                        port: None,
                        error: e,
                    })
                })?;

                let eth = ethernet::EthernetPacket::new(pkt);

                if eth.is_none() {
                    continue;
                }

                let eth = eth.unwrap();
                let header = ipv4::Ipv4Packet::new(eth.payload());

                if header.is_none() {
                    continue;
                }

                let header = header.unwrap();

                let device_ip = net::IpAddr::V4(header.get_source());
                let protocol = header.get_next_level_protocol();
                let payload = header.payload();

                if protocol != ip::IpNextHeaderProtocols::Tcp {
                    continue;
                }

                let tcp_packet = tcp::TcpPacket::new(payload);

                if tcp_packet.is_none() {
                    continue;
                }

                let tcp_packet = tcp_packet.unwrap();

                let destination_port = tcp_packet.get_destination();
                let matches_destination = destination_port == source_port;
                let flags: u8 = tcp_packet.get_flags();
                let sequence = tcp_packet.get_sequence();
                let is_syn_ack = flags == tcp::TcpFlags::SYN + tcp::TcpFlags::ACK;
                let is_expected_packet = matches_destination && is_syn_ack;

                if !is_expected_packet {
                    continue;
                }

                let device = devices.iter().find(|&d| d.ip == device_ip.to_string());

                if device.is_none() {
                    continue;
                }

                let device = device.unwrap();

                let port = u16::from_str(&tcp_packet.get_source().to_string());

                if port.is_err() {
                    continue;
                }

                let port = port.unwrap();

                // send rst packet to prevent SYN Flooding
                // https://en.wikipedia.org/wiki/SYN_flood
                // https://security.stackexchange.com/questions/128196/whats-the-advantage-of-sending-an-rst-packet-after-getting-a-response-in-a-syn
                let rst_packet = RSTPacket::new(
                    source_mac,
                    source_ipv4,
                    source_port,
                    net::Ipv4Addr::from_str(device.ip.as_str()).unwrap(),
                    util::MacAddr::from_str(device.mac.as_str()).unwrap(),
                    port,
                    sequence + 1,
                );

                let mut rst_sender = rst_packet_sender.lock().or_else(|e| {
                    Err(ScanError {
                        ip: None,
                        port: None,
                        error: Box::from(IOError::new(ErrorKind::Other, e.to_string())),
                    })
                })?;

                debug!("sending RST packet to {}:{}", device.ip, port);

                rst_sender.send(&rst_packet).or_else(|e| {
                    Err(ScanError {
                        ip: Some(device.ip.clone()),
                        port: Some(port.to_string()),
                        error: Box::from(e),
                    })
                })?;

                notifier
                    .send(ScanMessage::SYNScanResult(SYNScanResult {
                        device: device.to_owned(),
                        open_port: Port {
                            id: port,
                            service: String::from(""),
                        },
                    }))
                    .or_else(|e| {
                        Err(ScanError {
                            ip: Some(device.ip.clone()),
                            port: Some(port.to_string()),
                            error: Box::from(e),
                        })
                    })?;
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for SYNScanner
impl<'net> Scanner for SYNScanner<'net> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
        debug!("performing SYN scan on targets: {:?}", self.targets);

        debug!("starting syn packet reader");

        let (done_tx, done_rx) = mpsc::channel::<()>();
        let notifier = self.notifier.clone();
        let packet_sender = Arc::clone(&self.packet_sender);
        let targets = self.targets.clone();
        let interface = self.interface;
        let source_ipv4 = interface.ipv4;
        let source_mac = self.interface.mac;
        let ports = Arc::clone(&self.ports);
        let idle_timeout = self.idle_timeout.to_owned();
        let source_port = self.source_port.to_owned();

        let read_handle = self.read_packets(done_rx);

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<(), ScanError> {
            let mut scan_error: Option<ScanError> = None;

            for device in targets.iter() {
                let process_port = |port: u16| -> Result<(), ScanError> {
                    // throttle packet sending to prevent packet loss
                    thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                    debug!("scanning SYN target: {}:{}", device.ip, port);

                    let dest_ipv4 = net::Ipv4Addr::from_str(&device.ip).or_else(|e| {
                        Err(ScanError {
                            ip: Some(device.ip.clone()),
                            port: Some(port.to_string()),
                            error: Box::from(e),
                        })
                    })?;

                    let dest_mac = util::MacAddr::from_str(&device.mac).or_else(|e| {
                        Err(ScanError {
                            ip: Some(device.ip.clone()),
                            port: Some(port.to_string()),
                            error: Box::from(e),
                        })
                    })?;

                    let pkt_buf = SYNPacket::new(
                        source_mac,
                        source_ipv4,
                        source_port,
                        dest_ipv4,
                        dest_mac,
                        port,
                    );

                    // send info message to consumer
                    notifier
                        .send(ScanMessage::Info(Scanning {
                            ip: device.ip.clone(),
                            port: Some(port.to_string()),
                        }))
                        .or_else(|e| {
                            Err(ScanError {
                                ip: Some(device.ip.clone()),
                                port: Some(port.to_string()),
                                error: Box::from(e),
                            })
                        })?;

                    let mut sender = packet_sender.lock().or_else(|e| {
                        Err(ScanError {
                            ip: None,
                            port: None,
                            error: Box::from(IOError::new(ErrorKind::Other, e.to_string())),
                        })
                    })?;

                    // scan device @ port
                    sender.send(&pkt_buf).or_else(|e| {
                        Err(ScanError {
                            ip: Some(device.ip.clone()),
                            port: Some(port.to_string()),
                            error: Box::from(e),
                        })
                    })?;

                    Ok(())
                };

                if let Err(err) = ports.lazy_loop(process_port) {
                    scan_error = Some(err);
                }
            }

            thread::sleep(idle_timeout);

            notifier.send(ScanMessage::Done(())).or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(e),
                })
            })?;

            // ignore errors here as the thread may already be dead due to error
            // we'll catch any errors from that thread below and report
            let _ = done_tx.send(());

            let read_result = read_handle.join().or_else(|_| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(IOError::new(
                        ErrorKind::Other,
                        "encountered error during syn packet reading",
                    )),
                })
            })?;

            if let Some(err) = scan_error {
                return Err(err);
            }

            read_result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::packet::{arp, ethernet, ipv4, tcp};
    use std::collections::HashSet;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::network;
    use crate::scanners::DeviceWithPorts;
    use packet::arp::create_arp_reply;
    use packet::mocks::{MockPacketReader, MockPacketSender};
    use packet::syn::create_syn_reply;

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
        let devices: Vec<Device> = Vec::new();
        let ports = PortTargets::new(vec!["2000-8000".to_string()]);
        let (tx, _) = channel();

        let scanner = SYNScanner::new(
            &interface,
            receiver,
            sender,
            devices.clone(),
            ports,
            54321,
            idle_timeout,
            tx,
        );

        assert_eq!(scanner.targets, devices);
        assert_eq!(scanner.idle_timeout, idle_timeout);
        assert_eq!(scanner.source_port, 54321);
    }

    #[test]
    #[allow(warnings)]
    fn sends_and_reads_packets() {
        static mut PACKET: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];

        let interface = network::get_default_interface().unwrap();
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
            unsafe { &mut PACKET },
        );

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver
            .expect_next_packet()
            .returning(|| Ok(unsafe { &PACKET }));
        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));

        let idle_timeout = Duration::from_secs(2);
        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);
        let (tx, rx) = channel();

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
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
                    ScanMessage::Done(_) => {
                        break;
                    }
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

    #[test]
    #[allow(warnings)]
    fn ignores_unrelated_packets() {
        static mut SYN_PACKET1: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];
        static mut SYN_PACKET2: [u8; PKT_TOTAL_SYN_SIZE] = [0u8; PKT_TOTAL_SYN_SIZE];
        static mut ARP_PACKET: [u8; PKT_TOTAL_ARP_SIZE] = [0u8; PKT_TOTAL_ARP_SIZE];

        let interface = network::get_default_interface().unwrap();
        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();
        let device_ip = net::Ipv4Addr::from_str("192.168.0.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        // incorrect destination port
        create_syn_reply(
            device_mac.clone(),
            device_ip.clone(),
            2222,
            interface.mac,
            interface.ipv4,
            54322,
            unsafe { &mut SYN_PACKET1 },
        );

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
            unsafe { &mut SYN_PACKET2 },
        );

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
            unsafe { &mut ARP_PACKET },
        );

        receiver
            .expect_next_packet()
            .returning(|| Ok(unsafe { &ARP_PACKET }));

        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);

        let (tx, rx) = channel();

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

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
                    ScanMessage::Done(_) => {
                        break;
                    }
                    ScanMessage::SYNScanResult(result) => {
                        detected_devices.push(result.device);
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
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        let receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_receiver_clone = Arc::clone(&arc_receiver);
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let (tx, _rx) = channel();

        // Spawn a thread that will panic while holding the lock
        let _ = thread::spawn(move || {
            let _guard = arc_receiver_clone.lock().unwrap(); // Acquire the lock
            panic!("Simulated panic"); // Simulate a panic
        });

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

        let (_done_tx, done_rx) = channel();

        let handle = scanner.read_packets(done_rx);

        let result = handle.join();

        if result.is_err() {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
    }

    #[test]
    fn reports_error_on_packet_read_error() {
        let interface = network::get_default_interface().unwrap();
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver
            .expect_next_packet()
            .returning(|| Err(Box::from("oh no an error")));
        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let (tx, _rx) = channel();

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

        let (_done_tx, done_rx) = channel();

        let handle = scanner.read_packets(done_rx);

        let result = handle.join().unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn reports_error_on_notifier_send_errors() {
        let interface = network::get_default_interface().unwrap();
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver.expect_next_packet().returning(|| Ok(&[1]));
        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let (tx, rx) = channel();

        // this will cause an error when scanner tries to notify
        drop(rx);

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        let result = handle.join().unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn reports_error_on_packet_sender_lock_errors() {
        let interface = network::get_default_interface().unwrap();
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        let mut receiver = MockPacketReader::new();
        let sender = MockPacketSender::new();

        receiver.expect_next_packet().returning(|| Ok(&[1]));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let arc_sender_clone = Arc::clone(&arc_sender);
        let idle_timeout = Duration::from_secs(2);
        let (tx, rx) = channel();

        // Spawn a thread that will panic while holding the lock
        let _ = thread::spawn(move || {
            let _guard = arc_sender_clone.lock().unwrap(); // Acquire the lock
            panic!("Simulated panic"); // Simulate a panic
        });

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done(_) => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let result = handle.join().unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn reports_error_on_packet_send_errors() {
        let interface = network::get_default_interface().unwrap();
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver.expect_next_packet().returning(|| Ok(&[1]));
        sender
            .expect_send()
            .returning(|_| Err(Box::from("oh no a send error")));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let (tx, rx) = channel();

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done(_) => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let result = handle.join().unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn reports_errors_from_read_handle() {
        let interface = network::get_default_interface().unwrap();
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();

        let device = Device {
            hostname: "".to_string(),
            ip: device_ip.to_string(),
            mac: device_mac.to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        };

        let devices: Vec<Device> = vec![device.clone()];
        let ports = PortTargets::new(vec!["2222".to_string()]);

        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver
            .expect_next_packet()
            .returning(|| Err(Box::from("oh no a read error")));

        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let (tx, rx) = channel();

        let scanner = SYNScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            devices,
            ports,
            54321,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done(_) => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let result = handle.join().unwrap();

        assert!(result.is_err());
    }
}
