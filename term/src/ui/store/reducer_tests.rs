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
    ui::{
        colors::{Colors, Theme},
        store::{action::Action, effect::Effect, state::State},
    },
};

use super::Reducer;

fn setup() -> (State, Reducer) {
    let state = State::default();
    let reducer = Reducer::new();
    (state, reducer)
}

#[test]
fn test_ui_paused() {
    let (mut state, reducer) = setup();

    let effect = reducer.reduce(&mut state, Action::SetUIPaused(true));
    assert!(state.ui_paused);
    assert_eq!(effect, Effect::None);

    let effect = reducer.reduce(&mut state, Action::SetUIPaused(false));
    assert!(!state.ui_paused);
    assert_eq!(effect, Effect::None);
}

#[test]
fn test_set_error() {
    let (mut state, reducer) = setup();

    let effect =
        reducer.reduce(&mut state, Action::SetError(Some("error".to_string())));
    assert!(state.error.is_some());
    assert_eq!(effect, Effect::None);

    let effect = reducer.reduce(&mut state, Action::SetError(None));
    assert!(state.error.is_none());
    assert_eq!(effect, Effect::None);
}

#[test]
fn test_update_message() {
    let (mut state, reducer) = setup();
    let effect = reducer.reduce(
        &mut state,
        Action::UpdateMessage(Some("message".to_string())),
    );
    assert_eq!(state.message.unwrap(), "message".to_string());
    assert_eq!(effect, Effect::None);
}

#[test]
fn test_preview_theme() {
    let (mut state, reducer) = setup();
    let expected_colors = Colors::new(Theme::Emerald.to_palette(true), true);
    let effect =
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
    assert_eq!(effect, Effect::None);
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

    let effect = reducer
        .reduce(&mut state, Action::UpdateConfig(expected_config.clone()));
    assert_eq!(state.config, expected_config);
    assert_eq!(effect, Effect::SaveConfig(expected_config));
}

#[test]
fn test_update_all_devices() {
    let (mut state, reducer) = setup();

    let dev1 = Device {
        hostname: "dev1".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "dev1_vendor".to_string(),
    };

    let dev2 = Device {
        hostname: "dev2".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        open_ports: PortSet::new(),
        vendor: "dev2_vendor".to_string(),
    };

    let mut expected_devices = HashMap::new();
    expected_devices.insert(dev1.ip, dev1.clone());
    expected_devices.insert(dev2.ip, dev2.clone());

    let effect = reducer.reduce(
        &mut state,
        Action::UpdateAllDevices(expected_devices.clone()),
    );

    let devices = state.sorted_device_list;

    assert_eq!(devices.len(), 2);
    // sorted by IP so dev2 should be 1st
    assert_eq!(devices[0], dev2);
    assert_eq!(devices[1], dev1);
    assert_eq!(effect, Effect::None);
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
    };

    let effect = reducer.reduce(&mut state, Action::AddDevice(dev3.clone()));
    let devices = state.sorted_device_list;
    assert_eq!(devices, vec![dev3]);
    assert_eq!(effect, Effect::None);
}

#[test]
fn test_create_and_set_config() {
    let (mut state, reducer) = setup();
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();
    let mut config = Config::new(user, identity, cidr);
    config.id = "config_id".to_string();
    let effect =
        reducer.reduce(&mut state, Action::CreateAndSetConfig(config.clone()));
    assert_eq!(state.config.id, config.id);
    assert_eq!(effect, Effect::CreateConfig(config));
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
    };
    let port: u16 = 80;
    let effect = reducer.reduce(
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
    assert_eq!(effect, Effect::None);
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

    let effect = reducer.reduce(
        &mut state,
        Action::UpdateCommandOutput((cmd.clone(), output.clone())),
    );
    assert!(state.cmd_output.is_some());
    let info = state.cmd_output.clone().unwrap();
    assert_eq!(info.0, cmd);
    assert_eq!(info.1, output);
    assert_eq!(effect, Effect::None);

    let effect = reducer.reduce(&mut state, Action::ClearCommandOutput);
    assert!(state.cmd_output.is_none());
    assert_eq!(effect, Effect::None);
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
    };

    let port = Port {
        id: 80,
        service: "http".to_string(),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));
    let devices = state.sorted_device_list.clone();

    assert_eq!(devices.len(), 1);

    dev.open_ports.0.insert(port.clone());

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));
    let devices = state.sorted_device_list;

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0], dev);
    assert_eq!(devices[0].open_ports.0.len(), 1);
    assert!(devices[0].open_ports.0.contains(&port));
}
