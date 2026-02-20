use std::{
    collections::HashMap, net::Ipv4Addr, os::unix::process::ExitStatusExt,
    process::Output,
};

use pnet::util::MacAddr;
use r_lanlib::scanners::{Device, Port, PortSet};

use crate::{
    config::Config,
    ipc::message::Command,
    shell::traits::BrowseArgs,
    store::{action::Action, reducer::StoreReducer, state::State},
    ui::colors::{Colors, Theme},
};

use super::Reducer;

fn setup() -> (State, StoreReducer) {
    let state = State::default();
    (state, StoreReducer)
}

#[test]
fn test_ui_paused() {
    let (mut state, reducer) = setup();

    reducer.reduce(&mut state, Action::SetUIPaused(true));
    assert!(state.ui_paused);

    reducer.reduce(&mut state, Action::SetUIPaused(false));
    assert!(!state.ui_paused);
}

#[test]
fn test_set_error() {
    let (mut state, reducer) = setup();

    reducer.reduce(&mut state, Action::SetError(Some("error".to_string())));
    assert!(state.error.is_some());

    reducer.reduce(&mut state, Action::SetError(None));
    assert!(state.error.is_none());
}

#[test]
fn test_update_message() {
    let (mut state, reducer) = setup();
    reducer.reduce(
        &mut state,
        Action::UpdateMessage(Some("message".to_string())),
    );
    assert_eq!(state.message.unwrap(), "message".to_string());
}

#[test]
fn test_preview_theme() {
    let (mut state, reducer) = setup();
    let expected_colors = Colors::new(Theme::Emerald.to_palette(true), true);

    reducer.reduce(&mut state, Action::PreviewTheme(Theme::Emerald));
    assert_eq!(state.colors.border_color, expected_colors.border_color);
    assert_eq!(state.colors.buffer_bg, expected_colors.buffer_bg);
    assert_eq!(state.colors.row_header_bg, expected_colors.row_header_bg);
    assert_eq!(state.colors.input_editing, expected_colors.input_editing);
    assert_eq!(state.colors.header_text, expected_colors.header_text);
    assert_eq!(state.colors.text, expected_colors.text);
    assert_eq!(state.colors.gray, expected_colors.gray);
    assert_eq!(
        state.colors.selected_row_fg,
        expected_colors.selected_row_fg
    );
}

#[test]
fn test_update_config() {
    let (mut state, reducer) = setup();

    let expected_config = Config {
        cidr: "cidr".to_string(),
        default_ssh_identity: "id_rsa".to_string(),
        default_ssh_port: 2222,
        default_ssh_user: "user".to_string(),
        device_configs: HashMap::new(),
        id: "config_id".to_string(),
        ports: vec!["80".to_string(), "443".to_string()],
        theme: "Emerald".to_string(),
    };

    reducer.reduce(&mut state, Action::UpdateConfig(expected_config.clone()));
    assert_eq!(state.config, expected_config);
}

#[test]
fn test_remove_device_config() {
    let (mut state, reducer) = setup();

    let device_config = crate::config::DeviceConfig {
        id: "192.168.1.100".to_string(),
        ssh_port: 2222,
        ssh_identity_file: "/path/to/key".to_string(),
        ssh_user: "admin".to_string(),
    };

    // Add a device config
    reducer.reduce(
        &mut state,
        Action::UpdateDeviceConfig(device_config.clone()),
    );
    assert_eq!(state.config.device_configs.len(), 1);
    assert!(state.config.device_configs.contains_key("192.168.1.100"));

    // Remove the device config
    reducer.reduce(
        &mut state,
        Action::RemoveDeviceConfig("192.168.1.100".to_string()),
    );
    assert_eq!(state.config.device_configs.len(), 0);
    assert!(!state.config.device_configs.contains_key("192.168.1.100"));
}

#[test]
fn test_add_device() {
    let (mut state, reducer) = setup();

    let dev3 = Device {
        hostname: "dev3".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "dev3_vendor".to_string(),
        latency_ms: None,
    };

    reducer.reduce(&mut state, Action::AddDevice(dev3.clone()));
    let device = state.device_map.get(&dev3.ip).unwrap();

    assert_eq!(device, &dev3);
}

#[test]
fn test_set_command_in_progress() {
    let (mut state, reducer) = setup();
    let dev = Device {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        vendor: "dev_vendor".to_string(),
        open_ports: PortSet::new(),
        latency_ms: None,
    };
    let port: u16 = 80;
    reducer.reduce(
        &mut state,
        Action::SetCommandInProgress(Some(Command::Browse(BrowseArgs {
            device: dev.clone(),
            port,
            use_lynx: false,
        }))),
    );
    assert!(state.cmd_in_progress.is_some());
    let cmd = state.cmd_in_progress.unwrap();
    assert_eq!(
        cmd,
        Command::Browse(BrowseArgs {
            device: dev,
            port,
            use_lynx: false
        })
    );
}

#[test]
fn test_update_command_output() {
    let (mut state, reducer) = setup();
    let dev = Device {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        vendor: "dev_vendor".to_string(),
        open_ports: PortSet::new(),
        latency_ms: None,
    };
    let port: u16 = 80;
    let cmd = Command::Browse(BrowseArgs {
        device: dev.clone(),
        port,
        use_lynx: false,
    });

    let output = Output {
        status: ExitStatusExt::from_raw(0),
        stdout: "this is some output".as_bytes().to_vec(),
        stderr: vec![],
    };

    reducer.reduce(
        &mut state,
        Action::UpdateCommandOutput((cmd.clone(), output.clone())),
    );
    assert!(state.cmd_output.is_some());
    let info = state.cmd_output.clone().unwrap();
    assert_eq!(info.0, cmd);
    assert_eq!(info.1, output);

    reducer.reduce(&mut state, Action::ClearCommandOutput);
    assert!(state.cmd_output.is_none());
}

#[test]
fn test_add_device_records_latency_history() {
    let (mut state, reducer) = setup();

    let ip = Ipv4Addr::new(10, 10, 10, 1);
    let mut dev = Device {
        hostname: "dev".to_string(),
        ip,
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "vendor".to_string(),
        latency_ms: Some(5),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));
    assert_eq!(state.latency_history.get(&ip).unwrap(), &vec![5u64]);

    dev.latency_ms = Some(10);
    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));
    assert_eq!(state.latency_history.get(&ip).unwrap(), &vec![5u64, 10u64]);

    // latency_ms on the stored device should reflect the latest value
    assert_eq!(state.device_map.get(&ip).unwrap().latency_ms, Some(10));
}

#[test]
fn test_add_device_no_latency_does_not_append_history() {
    let (mut state, reducer) = setup();

    let ip = Ipv4Addr::new(10, 10, 10, 2);
    let dev = Device {
        hostname: "dev".to_string(),
        ip,
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "vendor".to_string(),
        latency_ms: None,
    };

    reducer.reduce(&mut state, Action::AddDevice(dev));
    assert!(!state.latency_history.contains_key(&ip));
}

#[test]
fn test_updates_device_with_new_info() {
    let (mut state, reducer) = setup();

    let mut dev = Device {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        vendor: "dev_vendor".to_string(),
        open_ports: PortSet::new(),
        latency_ms: None,
    };

    let port = Port {
        id: 80,
        service: "http".to_string(),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));

    assert_eq!(state.device_map.len(), 1);

    dev.open_ports.0.insert(port.clone());

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));

    let device = state.device_map.get(&dev.ip).unwrap();

    assert_eq!(state.device_map.len(), 1);
    assert_eq!(device, &dev);
    assert_eq!(device.open_ports.0.len(), 1);
    assert!(device.open_ports.0.contains(&port));
}
