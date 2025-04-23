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
    use mockall::mock;
    use mockall::predicate::*;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::network;
    use crate::packet;

    mock! {
        PacketSender {}

        impl packet::Sender for PacketSender {
            fn send(&mut self, packet: &[u8]) -> Result<(), std::io::Error>;
        }
    }

    mock! {
        PacketReceiver {}

        impl packet::Reader for PacketReceiver {
            fn next_packet(&mut self) -> Result<&'static [u8], std::io::Error>;
        }
    }

    #[test]
    fn test_new() {
        let interface = network::get_default_interface().unwrap();
        let sender: Arc<Mutex<dyn packet::Sender>> = Arc::new(Mutex::new(MockPacketSender::new()));
        let receiver: Arc<Mutex<dyn packet::Reader>> =
            Arc::new(Mutex::new(MockPacketReceiver::new()));
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
}
