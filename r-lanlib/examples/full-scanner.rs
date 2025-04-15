use std::{sync::mpsc, time::Duration};

use r_lanlib::{
    network, packet,
    scanners::{full_scanner::FullScanner, ScanMessage, Scanner},
    targets::{ips::IPTargets, ports::PortTargets},
};

fn main() {
    let interface = network::get_default_interface().expect("cannot find interface");
    let cidr = interface.cidr.clone();
    let wire = packet::wire::default(&interface).expect("failed to create wire");
    let ip_targets = IPTargets::new(vec![cidr]);
    let port_targets = PortTargets::new(vec![
        "22".to_string(),
        "80".to_string(),
        "443".to_string(),
        "2000-9000".to_string(),
    ]);
    let vendor = true;
    let host_names = true;
    let idle_timeout = Duration::from_millis(10000);
    let source_port: u16 = 54321;
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let scanner = FullScanner::new(
        &interface,
        wire.0,
        wire.1,
        ip_targets,
        port_targets,
        vendor,
        host_names,
        idle_timeout,
        tx,
        source_port,
    );

    let handle = scanner.scan();

    loop {
        let msg = rx.recv().expect("failed to poll for messages");

        match msg {
            ScanMessage::Done(_) => {
                println!("scanning complete");
                break;
            }
            _ => {
                println!("{:?}", msg)
            }
        }
    }

    if let Err(e) = handle.join() {
        println!("error: {:?}", e);
    }
}
