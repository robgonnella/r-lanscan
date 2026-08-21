// SYNScanner scans open ports on a known list of devices.
// In practice, the device list typically comes from a prior ARP scan.
// Here we use hardcoded devices for demonstration.
use pnet::util::MacAddr;
use r_lanlib::{
    network,
    scanners::{
        Device, PortSet, ScanMessage, Scanner, syn_scanner::SYNScanner,
    },
    targets::ports::PortTargets,
};
use std::{
    env,
    net::Ipv4Addr,
    sync::{Arc, mpsc},
    time::Duration,
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

    // Wire abstracts the raw packet reader/sender for the interface
    let wire =
        r_lanlib::wire::default(&interface).expect("failed to create wire");

    // Devices to scan — replace with output from ARPScanner in real usage
    let devices = vec![
        Device {
            hostname: "".to_string(),
            ip: Ipv4Addr::new(192, 168, 0, 1),
            mac: MacAddr::new(0x00, 0x00, 0x00, 0x00, 0x00, 0x01),
            vendor: "".to_string(),
            is_current_host: false,
            is_gateway: false,
            open_ports: PortSet::new(),
            latency_ms: None,
            response_ttl: None,
        },
        Device {
            hostname: "".to_string(),
            ip: Ipv4Addr::new(192, 168, 0, 2),
            mac: MacAddr::new(0x00, 0x00, 0x00, 0x00, 0x00, 0x02),
            vendor: "".to_string(),
            is_current_host: false,
            is_gateway: false,
            open_ports: PortSet::new(),
            latency_ms: None,
            response_ttl: None,
        },
        Device {
            hostname: "".to_string(),
            ip: Ipv4Addr::new(192, 168, 0, 3),
            mac: MacAddr::new(0x00, 0x00, 0x00, 0x00, 0x00, 0x03),
            vendor: "".to_string(),
            is_current_host: false,
            is_gateway: false,
            open_ports: PortSet::new(),
            latency_ms: None,
            response_ttl: None,
        },
    ];

    // Ports to check on each device — supports individual ports and ranges
    let port_targets = PortTargets::new(vec![
        "22".to_string(),
        "80".to_string(),
        "443".to_string(),
        "2000-9000".to_string(),
    ])
    .expect("failed to parse port targets");

    let idle_timeout = Duration::from_millis(10000);
    let source_port: u16 = 54321;

    // ScanMessage is sent on this channel as open ports are discovered
    let (tx, rx) = mpsc::channel::<ScanMessage>();

    let scanner = SYNScanner::builder()
        .interface(interface)
        .wire(wire)
        .targets(devices)
        .ports(port_targets)
        .source_port(source_port)
        .idle_timeout(idle_timeout)
        .notifier(tx)
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
            // SYNScanDevice is emitted for each device that has open ports;
            // the Device's open_ports field is populated
            ScanMessage::SYNScanDevice(result) => results.push(result),
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
