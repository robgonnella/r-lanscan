use super::*;

#[test]
fn returns_new_ip_targets() {
    let list = vec![
        String::from("192.128.28.1"),
        String::from("192.128.28.2"),
        String::from("192.128.28.3"),
    ];
    let targets = IPTargets::new(list).unwrap();
    assert!(!targets.0.is_empty());
}

#[test]
fn returns_port_target_len() {
    let list = vec![
        String::from("192.128.28.1"),
        String::from("192.128.28.2"),
        String::from("192.128.28.3"),
        String::from("192.128.28.4"),
        String::from("192.128.30.1"),
        String::from("192.128.30.2"),
    ];
    let targets = IPTargets::new(list).unwrap();
    assert_eq!(targets.len(), 6);
}

#[test]
fn lazy_loops_ips() {
    let list = vec![
        String::from("192.128.28.1"),
        String::from("192.128.28.2-192.128.28.4"),
        String::from("192.128.30.0/30"),
    ];

    let expected = [
        net::Ipv4Addr::from_str("192.128.28.1").unwrap(),
        net::Ipv4Addr::from_str("192.128.28.2").unwrap(),
        net::Ipv4Addr::from_str("192.128.28.3").unwrap(),
        net::Ipv4Addr::from_str("192.128.28.4").unwrap(),
        net::Ipv4Addr::from_str("192.128.30.1").unwrap(),
        net::Ipv4Addr::from_str("192.128.30.2").unwrap(),
    ];

    let targets = IPTargets::new(list).unwrap();

    let mut idx = 0;

    let assert_ips = |ip: net::Ipv4Addr| {
        assert_eq!(ip, expected[idx]);
        idx += 1;
        Ok(())
    };

    targets.lazy_loop(assert_ips).unwrap();
}

#[test]
fn returns_error_for_malformed_ip() {
    let list = vec![String::from("nope")];
    let result = IPTargets::new(list);
    assert!(result.is_err());
}

#[test]
fn returns_error_for_malformed_ip_with_slash() {
    let list = vec![String::from("no/pe")];
    let result = IPTargets::new(list);
    assert!(result.is_err());
}

#[test]
fn returns_error_for_malformed_range_start() {
    let list = vec![String::from("nope-192.168.0.3")];
    let result = IPTargets::new(list);
    assert!(result.is_err());
}

#[test]
fn returns_error_for_malformed_range_end() {
    let list = vec![String::from("192.168.0.4-nope")];
    let result = IPTargets::new(list);
    assert!(result.is_err());
}
