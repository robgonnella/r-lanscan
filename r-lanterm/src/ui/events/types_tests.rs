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

    let ssh = Command::SSH(device.clone(), device_config.clone());
    assert_eq!("ssh", ssh.to_string());

    let traceroute = Command::TRACEROUTE(device.clone());
    assert_eq!("traceroute", traceroute.to_string());

    let browse = Command::BROWSE(device, 80);
    assert_eq!("browse", browse.to_string());
}
