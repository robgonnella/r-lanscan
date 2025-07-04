use log::*;
use pnet::packet::{arp, ethernet, Packet};
use std::{
    io::{Error as IOError, ErrorKind},
    net,
    sync::{self, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    network::NetworkInterface,
    packet::{self, arp::ARPPacket, Reader, Sender},
    scanners::{Device, ScanError, Scanning},
    targets::ips::IPTargets,
};

use super::{heartbeat::HeartBeat, ScanMessage, Scanner};

// Data structure representing an ARP scanner
pub struct ARPScanner<'net> {
    interface: &'net NetworkInterface,
    packet_reader: Arc<Mutex<dyn Reader>>,
    packet_sender: Arc<Mutex<dyn Sender>>,
    targets: Arc<IPTargets>,
    source_port: u16,
    include_vendor: bool,
    include_host_names: bool,
    idle_timeout: Duration,
    notifier: sync::mpsc::Sender<ScanMessage>,
}

impl<'net> ARPScanner<'net> {
    pub fn new(
        interface: &'net NetworkInterface,
        packet_reader: Arc<Mutex<dyn Reader>>,
        packet_sender: Arc<Mutex<dyn Sender>>,
        targets: Arc<IPTargets>,
        source_port: u16,
        vendor: bool,
        host: bool,
        idle_timeout: Duration,
        notifier: sync::mpsc::Sender<ScanMessage>,
    ) -> Self {
        Self {
            interface,
            packet_reader,
            packet_sender,
            targets,
            source_port,
            include_vendor: vendor,
            include_host_names: host,
            idle_timeout,
            notifier,
        }
    }
}

impl<'net> ARPScanner<'net> {
    // Implements packet reading in a separate thread so we can send and
    // receive packets simultaneously
    fn read_packets(&self, done: sync::mpsc::Receiver<()>) -> JoinHandle<Result<(), ScanError>> {
        let packet_reader = Arc::clone(&self.packet_reader);
        let packet_sender = Arc::clone(&self.packet_sender);
        let include_host_names = self.include_host_names.clone();
        let include_vendor = self.include_vendor.clone();
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let source_port = self.source_port.to_owned();
        let notifier = self.notifier.clone();
        let (heartbeat_tx, heartbeat_rx) = sync::mpsc::channel::<()>();

        // since reading packets off the wire is a blocking operation, we
        // won't be able to detect a "done" signal if no packets are being
        // received as we'll be blocked on waiting for one to come it. To fix
        // this we send periodic "heartbeat" packets so we can continue to
        // check for "done" signals
        thread::spawn(move || {
            debug!("starting arp heartbeat thread");
            let heartbeat = HeartBeat::new(source_mac, source_ipv4, source_port, packet_sender);
            let interval = Duration::from_secs(1);
            loop {
                if let Ok(_) = heartbeat_rx.try_recv() {
                    debug!("stopping arp heartbeat");
                    break;
                }
                debug!("sending arp heartbeat");
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
                if let Ok(_) = done.try_recv() {
                    debug!("exiting arp packet reader");
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

                let header = arp::ArpPacket::new(eth.payload());

                if header.is_none() {
                    continue;
                }

                let header = header.unwrap();

                let op = header.get_operation();

                // Capture ANY ARP reply as it's an indiction that there's a
                // device on the network
                let is_expected_arp_packet = op == arp::ArpOperations::Reply;

                if !is_expected_arp_packet {
                    continue;
                }

                let ip4 = header.get_sender_proto_addr();
                let mac = eth.get_source().to_string();

                let notification_sender = notifier.clone();

                // use a separate thread here so we don't slow down packet
                // processing
                thread::spawn(move || {
                    let mut hostname: String = String::from("");
                    if include_host_names {
                        debug!("looking up hostname for {}", ip4.to_string());
                        if let Ok(host) = dns_lookup::lookup_addr(&ip4.into()) {
                            hostname = host;
                        }
                    }

                    let mut vendor = String::from("");
                    if include_vendor {
                        if let Some(vendor_data) = oui_data::lookup(&mac) {
                            vendor = vendor_data.organization().to_owned();
                        }
                    }

                    let _ = notification_sender.send(ScanMessage::ARPScanResult(Device {
                        hostname,
                        ip: ip4.to_string(),
                        mac,
                        vendor,
                        is_current_host: ip4 == source_ipv4,
                    }));
                });
            }

            Ok(())
        })
    }
}

// Implements the Scanner trait for ARPScanner
impl<'net> Scanner for ARPScanner<'net> {
    fn scan(&self) -> JoinHandle<Result<(), ScanError>> {
        debug!("performing ARP scan on targets: {:?}", self.targets);
        debug!("include_vendor: {}", self.include_vendor);
        debug!("include_host_names: {}", self.include_host_names);
        debug!("starting arp packet reader");
        let (done_tx, done_rx) = sync::mpsc::channel::<()>();
        let notifier = self.notifier.clone();
        let packet_sender = Arc::clone(&self.packet_sender);
        let idle_timeout = self.idle_timeout;
        let source_ipv4 = self.interface.ipv4;
        let source_mac = self.interface.mac;
        let targets = Arc::clone(&self.targets);

        let read_handle = self.read_packets(done_rx);

        // prevent blocking thread so messages can be freely sent to consumer
        thread::spawn(move || -> Result<(), ScanError> {
            let process_target = |target_ipv4: net::Ipv4Addr| {
                // throttle packet sending to prevent packet loss
                thread::sleep(packet::DEFAULT_PACKET_SEND_TIMING);

                debug!("scanning ARP target: {}", target_ipv4);

                let pkt_buf = ARPPacket::new(source_ipv4, source_mac, target_ipv4);

                // inform consumer we are scanning this target (ignore error on failure to notify)
                notifier
                    .send(ScanMessage::Info(Scanning {
                        ip: target_ipv4.to_string(),
                        port: None,
                    }))
                    .or_else(|e| {
                        Err(ScanError {
                            ip: Some(target_ipv4.to_string()),
                            port: None,
                            error: Box::from(e),
                        })
                    })?;

                let mut pkt_sender = packet_sender.lock().or_else(|e| {
                    Err(ScanError {
                        ip: Some(target_ipv4.to_string()),
                        port: None,
                        error: Box::from(IOError::new(ErrorKind::Other, e.to_string())),
                    })
                })?;

                // Send to the broadcast address
                pkt_sender.send(&pkt_buf).or_else(|e| {
                    Err(ScanError {
                        ip: Some(target_ipv4.to_string()),
                        port: None,
                        error: Box::from(e),
                    })
                })?;

                Ok(())
            };

            let mut scan_error: Option<ScanError> = None;

            if let Err(err) = targets.lazy_loop(process_target) {
                scan_error = Some(err);
            }

            thread::sleep(idle_timeout);

            notifier.send(ScanMessage::Done).or_else(|e| {
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
                    error: Box::from("error encountered in arp packet reading thread"),
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
    use pnet::{
        packet::{arp, ethernet, ipv4, tcp},
        util,
    };
    use std::str::FromStr;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::network;
    use crate::packet::arp::create_arp_reply;
    use crate::packet::mocks::{MockPacketReader, MockPacketSender};
    use crate::packet::syn::create_syn_reply;

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

        let scanner = ARPScanner::new(
            &interface,
            receiver,
            sender,
            targets,
            54321,
            true,
            true,
            idle_timeout,
            tx,
        );

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
            unsafe { &mut PACKET },
        );

        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver
            .expect_next_packet()
            .returning(|| Ok(unsafe { &PACKET }));
        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let targets = IPTargets::new(vec![device_ip.to_string()]);
        let (tx, rx) = channel();

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
            idle_timeout,
            tx,
        );

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

        let result = handle.join().unwrap().unwrap();
        assert_eq!(result, ());
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
                unsafe { &mut PACKET },
            );

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

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
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

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
            idle_timeout,
            tx,
        );

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
            .returning(|| Err(Box::from("oh no an error")));

        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
        let (tx, _rx) = channel();

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
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

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
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

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done => {
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
        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver.expect_next_packet().returning(|| Ok(&[1]));
        sender
            .expect_send()
            .returning(|_| Err(Box::from("oh no a send error")));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
        let (tx, rx) = channel();

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done => {
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
        let mut receiver = MockPacketReader::new();
        let mut sender = MockPacketSender::new();

        receiver
            .expect_next_packet()
            .returning(|| Err(Box::from("oh no a read error")));

        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver = Arc::new(Mutex::new(receiver));
        let arc_sender = Arc::new(Mutex::new(sender));
        let idle_timeout = Duration::from_secs(2);
        let targets = IPTargets::new(vec!["192.168.1.2".to_string()]);
        let (tx, rx) = channel();

        let scanner = ARPScanner::new(
            &interface,
            arc_receiver,
            arc_sender,
            targets,
            54321,
            true,
            true,
            idle_timeout,
            tx,
        );

        let handle = scanner.scan();

        loop {
            if let Ok(msg) = rx.recv() {
                match msg {
                    ScanMessage::Done => {
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
