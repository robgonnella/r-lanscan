use std::{
    env,
    sync::{Arc, mpsc},
    time::Duration,
};

use r_lanlib::{
    network::{self, get_default_gateway},
    packet,
    scanners::{Device, ScanMessage, Scanner, arp_scanner::ARPScanner},
    targets::ips::IPTargets,
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
    let interface = Arc::new(
        network::get_default_interface().expect("cannot find interface"),
    );
    let cidr = interface.cidr.clone();
    let wire =
        packet::wire::default(&interface).expect("failed to create wire");
    let ip_targets =
        IPTargets::new(vec![cidr]).expect("failed to parse IP targets");
    let vendor = true;
    let host_names = true;
    let idle_timeout = Duration::from_millis(10000);
    let source_port: u16 = 54321;
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let scanner = ARPScanner::builder()
        .interface(interface)
        .wire(wire)
        .gateway(get_default_gateway())
        .targets(ip_targets)
        .source_port(source_port)
        .include_vendor(vendor)
        .include_host_names(host_names)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .build()
        .unwrap();

    let mut results: Vec<Device> = Vec::new();

    let handle = scanner.scan().unwrap();

    loop {
        let msg = rx.recv().expect("failed to poll for messages");

        match msg {
            ScanMessage::Done => {
                println!("scanning complete");
                break;
            }
            ScanMessage::ARPScanDevice(result) => results.push(result),
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
