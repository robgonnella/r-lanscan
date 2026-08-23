use r_lanlib::MacAddr;
use r_lanlib::wire::DEFAULT_PACKET_SEND_TIMING;
use std::net::Ipv4Addr;
use std::str::FromStr;

use super::*;

fn default_args(debug: bool) -> Args {
    Args {
        debug,
        ports: vec!["80".to_string()],
        throttle: DEFAULT_PACKET_SEND_TIMING,
        from_arp_json: None,
        scan_interval: DEFAULT_SCAN_INTERVAL.into(),
    }
}

fn mock_interface() -> NetworkInterface {
    NetworkInterface {
        cidr: "192.168.1.1/24".to_string(),
        description: "test interface".to_string(),
        flags: 0,
        index: 0,
        ips: vec![],
        ipv4: Ipv4Addr::from_str("192.168.1.2").unwrap(),
        mac: MacAddr::default(),
        name: "test_interface".to_string(),
    }
}

#[test]
fn test_initialize_logger() {
    let args = default_args(false);
    initialize_logger(&args).unwrap();
}

#[test]
fn test_get_project_config_path() {
    let p = get_project_config_path().unwrap();
    assert_ne!(p, "");
}

#[test]
fn test_init() {
    let args = default_args(false);
    let interface = mock_interface();
    let (_config, _store) = init(&args, &interface).unwrap();
}

#[test]
fn scan_interval_defaults_to_the_shared_constant() {
    let args = Args::try_parse_from(["r-lanterm"]).unwrap();

    assert_eq!(Duration::from(args.scan_interval), DEFAULT_SCAN_INTERVAL);
}

#[test]
fn scan_interval_parses_a_human_duration() {
    let args =
        Args::try_parse_from(["r-lanterm", "--scan-interval", "90s"]).unwrap();

    assert_eq!(Duration::from(args.scan_interval), Duration::from_secs(90));
}

#[test]
fn scan_interval_rejects_zero() {
    // a zero interval would leave monitor() re-scanning with no pause
    let result = Args::try_parse_from(["r-lanterm", "--scan-interval", "0s"]);

    assert!(result.is_err());
}

#[test]
fn scan_interval_rejects_a_non_duration() {
    let result =
        Args::try_parse_from(["r-lanterm", "--scan-interval", "soonish"]);

    assert!(result.is_err());
}

#[test]
fn from_arp_json_parses_as_a_path() {
    let args =
        Args::try_parse_from(["r-lanterm", "--from-arp-json", "devices.json"])
            .unwrap();

    assert_eq!(args.from_arp_json, Some(PathBuf::from("devices.json")));
}
