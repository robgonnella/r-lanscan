use super::*;

#[test]
fn displays_command() {
    let device = Device {
        hostname: "".to_string(),
        ip: "".to_string(),
        is_current_host: false,
        mac: "".to_string(),
        vendor: "".to_string(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    let ssh = Command::Ssh(device.clone(), device_config.clone());
    assert_eq!("ssh", ssh.to_string());

    let traceroute = Command::TraceRoute(device.clone());
    assert_eq!("traceroute", traceroute.to_string());

    let browse = Command::Browse(BrowseArgs {
        device,
        port: 80,
        use_lynx: false,
    });
    assert_eq!("browse", browse.to_string());
}
