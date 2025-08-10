use std::{env, sync::mpsc, time::Duration};

use r_lanlib::{
    network, packet,
    scanners::{syn_scanner::SYNScanner, Device, SYNScanResult, ScanMessage, Scanner},
    targets::ports::PortTargets,
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
    let wire = packet::wire::default(&interface).expect("failed to create wire");
    let devices = vec![
        Device {
            hostname: "".to_string(),
            ip: "192.168.0.1".to_string(),
            mac: "00:00:00:00:00:01".to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        },
        Device {
            hostname: "".to_string(),
            ip: "192.168.0.2".to_string(),
            mac: "00:00:00:00:00:02".to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        },
        Device {
            hostname: "".to_string(),
            ip: "192.168.0.3".to_string(),
            mac: "00:00:00:00:00:03".to_string(),
            vendor: "".to_string(),
            is_current_host: false,
        },
    ];
    let port_targets = PortTargets::new(vec![
        "22".to_string(),
        "80".to_string(),
        "443".to_string(),
        "2000-9000".to_string(),
    ]);
    let idle_timeout = Duration::from_millis(10000);
    let source_port: u16 = 54321;
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let scanner = SYNScanner::new(
        &interface,
        wire.0,
        wire.1,
        devices,
        port_targets,
        source_port,
        idle_timeout,
        tx,
    );

    let mut results: Vec<SYNScanResult> = Vec::new();

    let handle = scanner.scan();

    loop {
        let msg = rx.recv().expect("failed to poll for messages");

        match msg {
            ScanMessage::Done => {
                println!("scanning complete");
                break;
            }
            ScanMessage::SYNScanResult(result) => results.push(result),
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
