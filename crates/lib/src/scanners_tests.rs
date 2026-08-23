use std::net::Ipv4Addr;
use std::str::FromStr;

use super::*;

fn device() -> Device {
    Device {
        hostname: "host".to_string(),
        ip: Ipv4Addr::new(192, 168, 1, 10),
        mac: MacAddr::from_str("00:11:22:33:44:55").unwrap(),
        vendor: "vendor".to_string(),
        is_current_host: false,
        is_gateway: true,
        open_ports: PortSet::from(HashSet::from([Port {
            id: 22,
            service: "ssh".to_string(),
        }])),
        latency_ms: Some(12),
        response_ttl: Some(64),
    }
}

#[test]
fn device_survives_a_json_round_trip() {
    let original = device();

    let json = serde_json::to_string(&vec![original.clone()]).unwrap();
    let parsed: Vec<Device> = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.len(), 1);

    // Device's PartialEq only compares ip + mac, so every field has to be
    // asserted individually for this to prove anything
    let parsed = &parsed[0];
    assert_eq!(parsed.hostname, original.hostname);
    assert_eq!(parsed.ip, original.ip);
    assert_eq!(parsed.mac, original.mac);
    assert_eq!(parsed.vendor, original.vendor);
    assert_eq!(parsed.is_current_host, original.is_current_host);
    assert_eq!(parsed.is_gateway, original.is_gateway);
    assert_eq!(parsed.open_ports.0, original.open_ports.0);
    assert_eq!(parsed.latency_ms, original.latency_ms);
    assert_eq!(parsed.response_ttl, original.response_ttl);
}

#[test]
fn device_deserializes_json_missing_optional_fields() {
    // json written by a build that predates open_ports, latency_ms and
    // response_ttl must still load, since scan output is now also an input
    let json = r#"[{
        "hostname": "host",
        "ip": "192.168.1.10",
        "mac": "00:11:22:33:44:55",
        "vendor": "vendor",
        "is_current_host": false,
        "is_gateway": true
    }]"#;

    let parsed: Vec<Device> = serde_json::from_str(json).unwrap();

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].ip, Ipv4Addr::new(192, 168, 1, 10));
    assert!(parsed[0].open_ports.0.is_empty());
    assert_eq!(parsed[0].latency_ms, None);
    assert_eq!(parsed[0].response_ttl, None);
}

#[test]
fn device_deserialization_rejects_a_malformed_mac() {
    let json = r#"[{
        "hostname": "host",
        "ip": "192.168.1.10",
        "mac": "not-a-mac",
        "vendor": "vendor",
        "is_current_host": false,
        "is_gateway": true
    }]"#;

    assert!(serde_json::from_str::<Vec<Device>>(json).is_err());
}
