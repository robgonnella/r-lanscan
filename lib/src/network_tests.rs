use super::*;

#[test]
fn returns_a_default_gateway() {
    // On any real machine running this test suite there must be a default
    // route configured; if there isn't the test environment itself is broken.
    let gw = get_default_gateway();
    assert!(gw.is_some(), "expected a default gateway to be detected");
}

#[test]
fn returns_error_for_bogus_interface_name() {
    let res = get_interface("noop");
    assert!(res.is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn returns_interface_by_name() {
    let res = get_interface("en0");
    assert!(res.is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn returns_interface_by_name() {
    let res = get_interface("eth0");
    assert!(res.is_ok());
}

#[test]
fn returns_default_interface() {
    let res = get_default_interface();
    assert!(res.is_ok());
}

#[test]
fn returns_an_available_port_on_system() {
    let res = get_available_port();
    assert!(res.is_ok());
}

#[test]
fn get_ip_and_cidr_from_interface() {
    let iface = pnet::datalink::interfaces()
        .into_iter()
        .find(|e| {
            e.is_up() && !e.is_loopback() && e.ips.iter().any(|i| i.is_ipv4())
        })
        .unwrap();
    let (ip, cidr) = get_interface_ipv4_and_cidr(&iface).unwrap();
    assert!(!ip.is_empty());
    assert!(!cidr.is_empty());
}
