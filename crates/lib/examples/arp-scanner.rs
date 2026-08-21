use std::{
    env,
    sync::{Arc, mpsc},
    time::Duration,
};

use r_lanlib::{
    network::{self, get_default_gateway},
    oui,
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

    // Detect the default network interface and derive the CIDR to scan
    let interface = Arc::new(
        network::get_default_interface().expect("cannot find interface"),
    );
    let cidr = interface.cidr.clone();

    // Wire abstracts the raw packet reader/sender for the interface
    let wire =
        r_lanlib::wire::default(&interface).expect("failed to create wire");

    // Scan all IPs in the interface's subnet
    let ip_targets =
        IPTargets::new(vec![cidr]).expect("failed to parse IP targets");

    let idle_timeout = Duration::from_millis(10000);
    let source_port: u16 = 54321;

    // ScanMessage is sent on this channel as devices are discovered
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    // Initialize the OUI database for MAC-to-vendor lookup.
    // Re-downloads IEEE CSV data if the cached copy is older than 30 days.
    let oui = oui::default(
        "r-lanscan-example",
        Duration::from_secs(60 * 60 * 24 * 30),
    )
    .unwrap();

    let scanner = ARPScanner::builder()
        .interface(interface)
        .wire(wire)
        // Optional: marks the gateway device with is_gateway=true in results
        .gateway(get_default_gateway())
        .targets(ip_targets)
        .source_port(source_port)
        .include_vendor(true)
        .include_host_names(true)
        .idle_timeout(idle_timeout)
        .notifier(tx)
        .oui(oui)
        .build()
        .unwrap();

    let mut results: Vec<Device> = Vec::new();

    // scan() spawns a background thread; join it after draining the channel
    let handle = scanner.scan().unwrap();

    loop {
        let msg = rx.recv().expect("failed to poll for messages");

        match msg {
            // Done signals the scan is complete and the thread is finishing
            ScanMessage::Done => {
                println!("scanning complete");
                break;
            }
            // ARPScanDevice is emitted for each discovered device
            ScanMessage::ARPScanDevice(result) => results.push(result),
            // Info carries scanning progress (current target IP)
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
