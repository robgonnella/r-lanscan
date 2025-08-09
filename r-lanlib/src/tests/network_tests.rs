use super::*;

#[test]
fn returns_none_for_bogus_interface_name() {
    let res = get_interface("noop");
    assert!(res.is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn returns_interface_by_name() {
    let res = get_interface("en0");
    assert!(res.is_some());
}

#[cfg(target_os = "linux")]
#[test]
fn returns_interface_by_name() {
    let res = get_interface("eth0");
    assert!(res.is_some());
}

#[test]
fn returns_default_interface() {
    let res = get_default_interface();
    assert!(res.is_some());
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
        .find(|e| e.is_up() && !e.is_loopback() && e.ips.iter().find(|i| i.is_ipv4()).is_some())
        .unwrap();
    let (ip, cidr) = get_interface_ipv4_and_cidr(&iface).unwrap();
    assert!(!ip.is_empty());
    assert!(!cidr.is_empty());
}
