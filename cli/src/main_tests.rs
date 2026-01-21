use mockall::mock;
use mpsc::channel;
use pnet::util::MacAddr;
use r_lanlib::scanners::{Port, SYNScanResult, Scanner};
use std::{
    net::Ipv4Addr,
    thread::{self, JoinHandle},
    time::Duration,
};

use super::*;

mock! {
    ArpScanner{}
    impl Scanner for ArpScanner {
        fn scan(&self) -> JoinHandle<r_lanlib::error::Result<()>>;
    }
}

mock! {
    SynScanner{}
    impl Scanner for SynScanner {
        fn scan(&self) -> JoinHandle<r_lanlib::error::Result<()>>;
    }
}

#[test]
fn prints_args() {
    let interface = network::get_default_interface().unwrap();

    let args = Args {
        json: false,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    print_args(&args, &interface);
}

#[test]
fn initializes_logger() {
    let args = Args {
        json: false,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    initialize_logger(&args).unwrap();
}

#[test]
fn prints_arp_table_results() {
    let args = Args {
        json: false,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        is_current_host: false,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        open_ports: PortSet::new(),
    };

    print_arp(&args, &vec![device]).unwrap();
}

#[test]
fn prints_arp_json_results() {
    let args = Args {
        json: true,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        is_current_host: false,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        open_ports: PortSet::new(),
    };

    print_arp(&args, &vec![device]).unwrap();
}

#[test]
fn prints_syn_table_results() {
    let args = Args {
        json: false,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    let port = Port {
        id: 22,
        service: "ssh".to_string(),
    };

    let mut open_ports = PortSet::new();
    open_ports.0.insert(port);

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        is_current_host: false,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        open_ports,
    };

    print_syn(&args, &vec![device]).unwrap();
}

#[test]
fn prints_syn_json_results() {
    let args = Args {
        json: true,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    let port = Port {
        id: 22,
        service: "ssh".to_string(),
    };

    let mut open_ports = PortSet::new();
    open_ports.0.insert(port);

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        is_current_host: false,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        open_ports,
    };

    print_syn(&args, &vec![device]).unwrap();
}

#[test]
fn performs_arp_scan() {
    let mut arp = MockArpScanner::new();

    let (tx, rx) = channel();

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        is_current_host: false,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        open_ports: PortSet::new(),
    };

    let device_clone = device.clone();

    thread::spawn(move || {
        let _ = tx.send(ScanMessage::ARPScanResult(device_clone));
        thread::sleep(Duration::from_millis(500));
        let _ = tx.send(ScanMessage::Done);
    });

    arp.expect_scan().returning(|| {
        let handle: JoinHandle<r_lanlib::error::Result<()>> = thread::spawn(|| Ok(()));
        handle
    });

    let result = process_arp(&arp, rx);

    assert!(result.is_ok());

    let (devices, _) = result.unwrap();

    assert_eq!(devices[0], device);
}

#[test]
fn performs_syn_scan() {
    let mut syn = MockSynScanner::new();

    let (tx, rx) = channel();

    let mut ports = PortSet::new();
    ports.0.insert(Port {
        id: 22,
        service: "ssh".to_string(),
    });

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        is_current_host: false,
        mac: MacAddr::default(),
        vendor: "vendor".to_string(),
        open_ports: ports,
    };

    let device_clone = device.clone();

    thread::spawn(move || {
        let _ = tx.send(ScanMessage::SYNScanResult(SYNScanResult {
            device: device_clone,
        }));
        thread::sleep(Duration::from_millis(500));
        let _ = tx.send(ScanMessage::Done);
    });

    syn.expect_scan().returning(|| {
        let handle: JoinHandle<r_lanlib::error::Result<()>> = thread::spawn(|| Ok(()));
        handle
    });

    let result = process_syn(&syn, vec![device.clone()], rx);

    assert!(result.is_ok());

    let devices = result.unwrap();

    assert_eq!(devices[0], device);
}
