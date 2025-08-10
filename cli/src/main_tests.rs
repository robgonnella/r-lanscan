use mockall::mock;
use mpsc::channel;
use pnet::util::MacAddr;
use r_lanlib::scanners::{Port, SYNScanResult, Scanner};
use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use super::*;

mock! {
    ArpScanner{}
    impl Scanner for ArpScanner {
        fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
    }
}

mock! {
    SynScanner{}
    impl Scanner for SynScanner {
        fn scan(&self) -> JoinHandle<Result<(), ScanError>>;
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
        interface: "interface_name".to_string(),
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
        interface: "interface_name".to_string(),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    initialize_logger(&args);
}

#[test]
fn prints_arp_table_results() {
    let args = Args {
        json: false,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: "interface_name".to_string(),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    let device = Device {
        hostname: "hostname".to_string(),
        ip: "192.168.1.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        vendor: "vendor".to_string(),
    };

    print_arp(&args, &vec![device]);
}

#[test]
fn prints_arp_json_results() {
    let args = Args {
        json: true,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: "interface_name".to_string(),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
    };

    let device = Device {
        hostname: "hostname".to_string(),
        ip: "192.168.1.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        vendor: "vendor".to_string(),
    };

    print_arp(&args, &vec![device]);
}

#[test]
fn prints_syn_table_results() {
    let args = Args {
        json: false,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: "interface_name".to_string(),
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

    let mut open_ports = HashSet::new();
    open_ports.insert(port);

    let device = DeviceWithPorts {
        hostname: "hostname".to_string(),
        ip: "192.168.1.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        vendor: "vendor".to_string(),
        open_ports,
    };

    print_syn(&args, &vec![device]);
}

#[test]
fn prints_syn_json_results() {
    let args = Args {
        json: true,
        arp_only: false,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: "interface_name".to_string(),
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

    let mut open_ports = HashSet::new();
    open_ports.insert(port);

    let device = DeviceWithPorts {
        hostname: "hostname".to_string(),
        ip: "192.168.1.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        vendor: "vendor".to_string(),
        open_ports,
    };

    print_syn(&args, &vec![device]);
}

#[test]
fn performs_arp_scan() {
    let mut arp = MockArpScanner::new();

    let (tx, rx) = channel();

    let device = Device {
        hostname: "hostname".to_string(),
        ip: "192.168.1.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        vendor: "vendor".to_string(),
    };

    let device_clone = device.clone();

    thread::spawn(move || {
        let _ = tx.send(ScanMessage::ARPScanResult(device_clone));
        thread::sleep(Duration::from_millis(500));
        let _ = tx.send(ScanMessage::Done);
    });

    arp.expect_scan().returning(|| {
        let handle: JoinHandle<Result<(), ScanError>> = thread::spawn(|| Ok(()));
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

    let device = Device {
        hostname: "hostname".to_string(),
        ip: "192.168.1.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        vendor: "vendor".to_string(),
    };

    let port = Port {
        id: 22,
        service: "ssh".to_string(),
    };

    let device_clone = device.clone();
    let port_clone = port.clone();

    thread::spawn(move || {
        let _ = tx.send(ScanMessage::SYNScanResult(SYNScanResult {
            device: device_clone,
            open_port: port_clone,
        }));
        thread::sleep(Duration::from_millis(500));
        let _ = tx.send(ScanMessage::Done);
    });

    syn.expect_scan().returning(|| {
        let handle: JoinHandle<Result<(), ScanError>> = thread::spawn(|| Ok(()));
        handle
    });

    let result = process_syn(&syn, vec![device.clone()], rx);

    assert!(result.is_ok());

    let devices = result.unwrap();

    let mut expected_open_ports = HashSet::new();
    expected_open_ports.insert(port);

    let expected_device = DeviceWithPorts {
        hostname: device.hostname,
        ip: device.ip,
        is_current_host: device.is_current_host,
        mac: device.mac,
        vendor: device.vendor,
        open_ports: expected_open_ports,
    };

    assert_eq!(devices[0], expected_device);
}
