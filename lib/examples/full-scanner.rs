use std::{env, sync::mpsc, time::Duration};

use r_lanlib::{
    network, packet,
    scanners::{
        Device, ScanMessage, Scanner,
        full_scanner::{FullScanner, FullScannerArgs},
    },
    targets::{ips::IPTargets, ports::PortTargets},
};

fn is_root() -> bool {
    match env::var("USER") {
        Ok(val) => val == "root",
        Err(_e) => false,
    }
}

fn main() {
    if !is_root() {
        panic!("permission denied: must run with root privileges");
    }
    let interface = network::get_default_interface().expect("cannot find interface");
    let cidr = interface.cidr.clone();
    let wire = packet::wire::default(&interface).expect("failed to create wire");
    let ip_targets = IPTargets::new(vec![cidr]).expect("failed to parse IP targets");
    let port_targets =
        PortTargets::new(vec!["1-65535".to_string()]).expect("failed to parse port targets");
    let vendor = true;
    let host_names = true;
    let idle_timeout = Duration::from_millis(10000);
    let source_port: u16 = 54321;
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let scanner = FullScanner::new(FullScannerArgs {
        interface: &interface,
        packet_reader: wire.0,
        packet_sender: wire.1,
        targets: ip_targets,
        ports: port_targets,
        include_vendor: vendor,
        include_host_names: host_names,
        idle_timeout,
        notifier: tx,
        source_port,
    });

    let mut results: Vec<Device> = Vec::new();

    let handle = scanner.scan();

    loop {
        let msg = rx.recv().expect("failed to poll for messages");

        match msg {
            ScanMessage::Done => {
                println!("scanning complete");
                break;
            }
            ScanMessage::SYNScanDevice(device) => results.push(device),
            _ => {
                println!("{:?}", msg)
            }
        }
    }

    if let Err(e) = handle.join() {
        panic!("error: {:?}", e);
    }

    println!("results: {:?}", results);
}
