use mockall::mock;
use mpsc::channel;
use r_lanlib::{
    error::Result,
    scanners::{Port, PortSet, Scanner},
    wire::DEFAULT_PACKET_SEND_TIMING,
};
use std::{
    io::Write,
    net::Ipv4Addr,
    thread::{self, JoinHandle},
    time::Duration,
};

use super::*;

mock! {
    ArpScanner{}
    impl Scanner for ArpScanner {
        fn scan(&self) -> Result<JoinHandle<r_lanlib::error::Result<()>>>;
    }
}

mock! {
    SynScanner{}
    impl Scanner for SynScanner {
        fn scan(&self) -> Result<JoinHandle<r_lanlib::error::Result<()>>>;
    }
}

#[test]
fn prints_args() {
    let interface = network::get_default_interface().unwrap();

    let args = Args {
        json: false,
        from_arp_json: None,
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
        throttle: DEFAULT_PACKET_SEND_TIMING,
    };

    print_args(&args, &interface);
}

#[test]
fn initializes_logger() {
    let args = Args {
        json: false,
        from_arp_json: None,
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
        throttle: DEFAULT_PACKET_SEND_TIMING,
    };

    initialize_logger(&args).unwrap();
}

#[test]
fn prints_arp_table_results() {
    let args = Args {
        json: false,
        from_arp_json: None,
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
        throttle: DEFAULT_PACKET_SEND_TIMING,
    };

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        vendor: "vendor".to_string(),
        ..Device::default()
    };

    print_arp(&args, &vec![device]).unwrap();
}

#[test]
fn prints_arp_json_results() {
    let args = Args {
        json: true,
        from_arp_json: None,
        arp_only: true,
        debug: false,
        host_names: true,
        idle_timeout_ms: 2000,
        interface: Some("interface_name".to_string()),
        ports: vec!["22".to_string()],
        quiet: false,
        source_port: 54321,
        targets: vec!["192.168.1.1".to_string()],
        vendor: true,
        throttle: DEFAULT_PACKET_SEND_TIMING,
    };

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        vendor: "vendor".to_string(),
        ..Device::default()
    };

    print_arp(&args, &vec![device]).unwrap();
}

#[test]
fn prints_syn_table_results() {
    let args = Args {
        json: false,
        from_arp_json: None,
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
        throttle: DEFAULT_PACKET_SEND_TIMING,
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
        vendor: "vendor".to_string(),
        open_ports,
        ..Device::default()
    };

    let devices = HashMap::from([(device.ip, device)]);
    print_syn(&args, &devices).unwrap();
}

#[test]
fn prints_syn_json_results() {
    let args = Args {
        json: true,
        from_arp_json: None,
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
        throttle: DEFAULT_PACKET_SEND_TIMING,
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
        vendor: "vendor".to_string(),
        open_ports,
        ..Device::default()
    };

    let devices = HashMap::from([(device.ip, device)]);
    print_syn(&args, &devices).unwrap();
}

#[test]
fn performs_arp_scan() {
    let mut arp = MockArpScanner::new();

    let (tx, rx) = channel();

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 1),
        vendor: "vendor".to_string(),
        ..Device::default()
    };

    let device_clone = device.clone();

    thread::spawn(move || {
        let _ = tx.send(ScanMessage::ARPScanDevice(device_clone));
        thread::sleep(Duration::from_millis(500));
        let _ = tx.send(ScanMessage::Done);
    });

    arp.expect_scan().returning(|| {
        let handle: JoinHandle<r_lanlib::error::Result<()>> =
            thread::spawn(|| Ok(()));
        Ok(handle)
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
        vendor: "vendor".to_string(),
        open_ports: ports,
        ..Device::default()
    };

    let device_clone = device.clone();

    thread::spawn(move || {
        let _ = tx.send(ScanMessage::SYNScanDevice(device_clone));
        thread::sleep(Duration::from_millis(500));
        let _ = tx.send(ScanMessage::Done);
    });

    syn.expect_scan().returning(|| {
        let handle: JoinHandle<r_lanlib::error::Result<()>> =
            thread::spawn(|| Ok(()));
        Ok(handle)
    });

    let result = process_syn(&syn, vec![device.clone()], rx);

    assert!(result.is_ok());

    let devices = result.unwrap();

    assert_eq!(devices.get(&device.ip), Some(&device));
}

fn write_json(contents: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn from_arp_json_conflicts_with_arp_only() {
    // seeding from a file skips the ARP scan, so --arp-only would leave
    // nothing to do and silently print nothing
    let result = Args::try_parse_from([
        "r-lancli",
        "--from-arp-json",
        "devices.json",
        "--arp-only",
    ]);

    assert!(result.is_err());
}

#[test]
fn from_arp_json_conflicts_with_targets() {
    // the seeded device list replaces the target list entirely
    let result = Args::try_parse_from([
        "r-lancli",
        "--from-arp-json",
        "devices.json",
        "--targets",
        "192.168.1.0/24",
    ]);

    assert!(result.is_err());
}

#[test]
fn from_arp_json_parses_on_its_own() {
    let args =
        Args::try_parse_from(["r-lancli", "--from-arp-json", "devices.json"])
            .unwrap();

    assert_eq!(args.from_arp_json, Some(PathBuf::from("devices.json")));
}

#[test]
fn loads_devices_from_json() {
    let file = write_json(
        r#"[{
            "hostname": "host",
            "ip": "192.168.1.10",
            "mac": "00:11:22:33:44:55",
            "vendor": "vendor",
            "is_current_host": false,
            "is_gateway": false,
            "open_ports": [],
            "latency_ms": null,
            "response_ttl": null
        }]"#,
    );

    let devices = load_devices_from_json(file.path()).unwrap();

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].ip, Ipv4Addr::new(192, 168, 1, 10));
    assert_eq!(devices[0].hostname, "host");
}

#[test]
fn load_devices_from_json_names_a_missing_file() {
    let err = load_devices_from_json(Path::new("/nope/missing.json"))
        .unwrap_err()
        .to_string();

    assert!(err.contains("failed to read"), "unexpected error: {err}");
    assert!(
        err.contains("/nope/missing.json"),
        "unexpected error: {err}"
    );
}

#[test]
fn load_devices_from_json_names_a_malformed_file() {
    let file = write_json("not json at all");

    let err = load_devices_from_json(file.path()).unwrap_err().to_string();

    assert!(err.contains("failed to parse"), "unexpected error: {err}");
    assert!(
        err.contains(&file.path().display().to_string()),
        "unexpected error: {err}"
    );
}

#[test]
fn load_devices_from_json_rejects_an_empty_device_list() {
    // otherwise this falls through to a SYN scan with zero targets and
    // prints an empty report as though the scan succeeded
    let file = write_json("[]");

    let err = load_devices_from_json(file.path()).unwrap_err().to_string();

    assert!(err.contains("no devices found"), "unexpected error: {err}");
}

fn parse(flags: &[&str]) -> Args {
    let mut argv = vec!["r-lancli"];
    argv.extend_from_slice(flags);
    Args::try_parse_from(argv).unwrap()
}

#[test]
fn log_filter_defaults_to_info() {
    assert_eq!(log_filter(&parse(&[])), simplelog::LevelFilter::Info);
}

#[test]
fn log_filter_is_error_when_quiet() {
    assert_eq!(
        log_filter(&parse(&["--quiet"])),
        simplelog::LevelFilter::Error
    );
}

#[test]
fn log_filter_is_error_when_json() {
    // info logs go to stdout, so they would corrupt a redirected report
    assert_eq!(
        log_filter(&parse(&["--json"])),
        simplelog::LevelFilter::Error
    );
}

#[test]
fn log_filter_lets_debug_override_json() {
    assert_eq!(
        log_filter(&parse(&["--json", "--debug"])),
        simplelog::LevelFilter::Debug
    );
}

#[test]
fn log_filter_is_debug_when_debug() {
    assert_eq!(
        log_filter(&parse(&["--debug"])),
        simplelog::LevelFilter::Debug
    );
}

#[test]
fn log_filter_lets_quiet_win_over_debug() {
    assert_eq!(
        log_filter(&parse(&["--quiet", "--debug"])),
        simplelog::LevelFilter::Error
    );
}

#[test]
fn arp_report_prints_for_a_plain_full_scan() {
    assert!(should_print_arp_report(&parse(&[])));
}

#[test]
fn arp_report_is_skipped_for_a_json_full_scan() {
    // otherwise stdout gets the arp array followed by the syn array, which
    // is not valid json
    assert!(!should_print_arp_report(&parse(&["--json"])));
}

#[test]
fn arp_report_is_skipped_for_a_quiet_full_scan() {
    assert!(!should_print_arp_report(&parse(&["--quiet"])));
}

#[test]
fn arp_report_prints_when_arp_only() {
    // nothing follows it, so it is the only report there is
    assert!(should_print_arp_report(&parse(&["--arp-only"])));
    assert!(should_print_arp_report(&parse(&["--arp-only", "--json"])));
    assert!(should_print_arp_report(&parse(&["--arp-only", "--quiet"])));
}
