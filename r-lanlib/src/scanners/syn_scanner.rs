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

use super::{Device, Port, PortStatus, SYNScanResult, ScanMessage, Scanner};

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
                    error: Box::from(IOError::new(ErrorKind::Other, e.to_string())),
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
                        error: Box::new(e),
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
                            status: PortStatus::Open,
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

                ports.lazy_loop(process_port)?;
            }

            thread::sleep(idle_timeout);

            done_tx.send(()).or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(e),
                })
            })?;

            notifier.send(ScanMessage::Done(())).or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: None,
                    error: Box::from(e),
                })
            })?;

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

            read_result
        })
    }
}

#[cfg(test)]
mod tests {
    use packet::syn::create_syn_reply;

    use super::*;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::network;
    use crate::packet::{MockPacketReader, MockPacketSender};

    #[test]
    fn test_new() {
        let interface = network::get_default_interface().unwrap();
        let sender: Arc<Mutex<dyn packet::Sender>> = Arc::new(Mutex::new(MockPacketSender::new()));
        let receiver: Arc<Mutex<dyn packet::Reader>> =
            Arc::new(Mutex::new(MockPacketReader::new()));
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
    fn test_sends_and_read_packets() {
        let interface = network::get_default_interface().unwrap();
        let device_ip = net::Ipv4Addr::from_str("192.168.1.2").unwrap();
        let device_mac = util::MacAddr::default();
        let device_port = 2222;

        let packet = create_syn_reply(
            device_mac,
            device_ip,
            device_port,
            interface.mac,
            interface.ipv4,
            54321,
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

        receiver.expect_next_packet().returning(|| Ok(packet));
        sender.expect_send().returning(|_| Ok(()));

        let arc_receiver: Arc<Mutex<dyn packet::Reader>> = Arc::new(Mutex::new(receiver));
        let arc_sender: Arc<Mutex<dyn packet::Sender>> = Arc::new(Mutex::new(sender));

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

        if let Ok(msg) = rx.recv() {
            match msg {
                ScanMessage::SYNScanResult(d) => {
                    assert_eq!(d.device.hostname, device.hostname);
                    assert_eq!(d.device.ip, device.ip);
                    assert_eq!(d.device.mac, device.mac);
                    assert_eq!(d.device.vendor, device.vendor);
                    assert_eq!(d.device.is_current_host, device.is_current_host);
                    assert_eq!(d.open_port.id, device_port);
                }
                _ => {}
            }
        }

        let result = handle.join().unwrap().unwrap();
        assert_eq!(result, ());
    }
}
