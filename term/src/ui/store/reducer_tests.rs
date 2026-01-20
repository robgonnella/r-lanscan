use std::{
    collections::{HashMap, HashSet},
    fs,
    net::Ipv4Addr,
    os::unix::process::ExitStatusExt,
    process::Output,
    sync::{Arc, Mutex},
};

use nanoid::nanoid;
use pnet::util::MacAddr;
use r_lanlib::scanners::{Device, DeviceWithPorts, Port};

use crate::{
    config::{Config, ConfigManager, DeviceConfig},
    events::types::{BrowseArgs, Command},
    ui::{
        colors::{Colors, Theme},
        store::{
            action::Action,
            state::{State, ViewID},
        },
    },
};

use super::Reducer;

fn setup() -> (State, Reducer, String) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let conf_manager = Arc::new(Mutex::new(
        ConfigManager::new(user, identity, tmp_path.as_str()).unwrap(),
    ));

    let state = State::default();
    let reducer = Reducer::new(conf_manager);

    (state, reducer, tmp_path)
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_ui_paused() {
    let (mut state, reducer, conf_path) = setup();

    reducer.reduce(&mut state, Action::SetUIPaused(true));
    assert!(state.ui_paused);

    reducer.reduce(&mut state, Action::SetUIPaused(false));
    assert!(!state.ui_paused);
    tear_down(conf_path);
}

#[test]
fn test_set_error() {
    let (mut state, reducer, conf_path) = setup();

    reducer.reduce(&mut state, Action::SetError(Some("error".to_string())));
    assert!(state.error.is_some());

    reducer.reduce(&mut state, Action::SetError(None));
    assert!(state.error.is_none());

    tear_down(conf_path);
}

#[test]
fn test_toggle_view_select() {
    let (mut state, reducer, conf_path) = setup();
    reducer.reduce(&mut state, Action::ToggleViewSelect);
    assert!(state.render_view_select);
    tear_down(conf_path);
}

#[test]
fn test_update_view() {
    let (mut state, reducer, conf_path) = setup();
    reducer.reduce(&mut state, Action::UpdateView(ViewID::Config));
    assert_eq!(state.view_id, ViewID::Config);
    tear_down(conf_path);
}

#[test]
fn test_update_message() {
    let (mut state, reducer, conf_path) = setup();
    reducer.reduce(
        &mut state,
        Action::UpdateMessage(Some("message".to_string())),
    );
    assert_eq!(state.message.unwrap(), "message".to_string());
    tear_down(conf_path);
}

#[test]
fn test_preview_theme() {
    let (mut state, reducer, conf_path) = setup();
    let expected_colors = Colors::new(Theme::Emerald.to_palette(true), true);
    reducer.reduce(&mut state, Action::PreviewTheme(Theme::Emerald));
    assert_eq!(state.colors.border_color, expected_colors.border_color);
    assert_eq!(state.colors.buffer_bg, expected_colors.buffer_bg);
    assert_eq!(state.colors.header_bg, expected_colors.header_bg);
    assert_eq!(state.colors.header_fg, expected_colors.header_fg);
    assert_eq!(state.colors.input_editing, expected_colors.input_editing);
    assert_eq!(state.colors.label, expected_colors.label);
    assert_eq!(state.colors.row_bg, expected_colors.row_bg);
    assert_eq!(state.colors.row_fg, expected_colors.row_fg);
    assert_eq!(state.colors.scroll_bar_fg, expected_colors.scroll_bar_fg);
    assert_eq!(
        state.colors.selected_row_fg,
        expected_colors.selected_row_fg
    );
    tear_down(conf_path);
}

#[test]
fn test_update_config() {
    let (mut state, reducer, conf_path) = setup();

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

    tear_down(conf_path);
}

#[test]
fn test_update_all_devices() {
    let (mut state, reducer, conf_path) = setup();

    let dev1 = DeviceWithPorts {
        hostname: "dev1".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: HashSet::new(),
        vendor: "dev1_vendor".to_string(),
    };

    let dev2 = DeviceWithPorts {
        hostname: "dev2".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        open_ports: HashSet::new(),
        vendor: "dev2_vendor".to_string(),
    };

    let expected_devices = vec![dev1.clone(), dev2.clone()];

    reducer.reduce(
        &mut state,
        Action::UpdateAllDevices(expected_devices.clone()),
    );
    assert_eq!(state.devices, expected_devices);

    tear_down(conf_path);
}

#[test]
fn test_add_device() {
    let (mut state, reducer, conf_path) = setup();

    let dev3 = DeviceWithPorts {
        hostname: "dev3".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: HashSet::new(),
        vendor: "dev3_vendor".to_string(),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev3.clone()));
    assert_eq!(state.devices, vec![dev3.clone()]);
    tear_down(conf_path);
}

#[test]
fn test_set_config() {
    let (mut state, reducer, conf_path) = setup();
    reducer.reduce(&mut state, Action::SetConfig("default".to_string()));
    assert_eq!(state.config.id, "default");
    tear_down(conf_path);
}

#[test]
fn test_create_and_set_config() {
    let (mut state, reducer, conf_path) = setup();
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let mut config = Config::new(user, identity);
    config.id = "config_id".to_string();
    reducer.reduce(&mut state, Action::CreateAndSetConfig(config.clone()));
    assert_eq!(state.config.id, config.id);
    tear_down(conf_path);
}

#[test]
fn test_update_selected_device() {
    let (mut state, reducer, conf_path) = setup();

    let dev1 = DeviceWithPorts {
        hostname: "dev1".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: HashSet::new(),
        vendor: "dev1_vendor".to_string(),
    };

    let dev2 = DeviceWithPorts {
        hostname: "dev2".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        open_ports: HashSet::new(),
        vendor: "dev2_vendor".to_string(),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev1.clone()));
    reducer.reduce(&mut state, Action::AddDevice(dev2.clone()));
    reducer.reduce(
        &mut state,
        Action::UpdateAllDevices(vec![dev1.clone(), dev2.clone()]),
    );
    reducer.reduce(&mut state, Action::UpdateSelectedDevice(dev2.ip));
    assert!(state.selected_device.is_some());
    let selected = state.selected_device.unwrap();
    assert_eq!(selected.mac, dev2.mac);
    tear_down(conf_path);
}

#[test]
fn test_update_device_config() {
    let (mut state, reducer, conf_path) = setup();

    let dev = DeviceWithPorts {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        open_ports: HashSet::new(),
        vendor: "dev_vendor".to_string(),
    };

    let dev_config = DeviceConfig {
        id: dev.mac.to_string(),
        ssh_identity_file: "id_rsa".to_string(),
        ssh_port: 2222,
        ssh_user: "dev_user".to_string(),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));
    reducer.reduce(&mut state, Action::UpdateAllDevices(vec![dev.clone()]));
    reducer.reduce(&mut state, Action::UpdateDeviceConfig(dev_config.clone()));
    reducer.reduce(&mut state, Action::UpdateSelectedDevice(dev.ip));

    assert!(state.selected_device_config.is_some());
    let selected = state.selected_device_config.unwrap();
    assert_eq!(selected.id, dev_config.id);
    assert_eq!(selected.ssh_port, dev_config.ssh_port);

    tear_down(conf_path);
}

#[test]
fn test_set_command_in_progress() {
    let (mut state, reducer, conf_path) = setup();
    let dev = Device {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        vendor: "dev_vendor".to_string(),
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
    tear_down(conf_path);
}

#[test]
fn test_update_command_output() {
    let (mut state, reducer, conf_path) = setup();
    let dev = Device {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        vendor: "dev_vendor".to_string(),
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
    tear_down(conf_path);
}

#[test]
fn test_updates_device_with_new_info() {
    let (mut state, reducer, conf_path) = setup();

    let mut dev = DeviceWithPorts {
        hostname: "dev".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: false,
        vendor: "dev_vendor".to_string(),
        open_ports: HashSet::new(),
    };

    let port = Port {
        id: 80,
        service: "http".to_string(),
    };

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));

    assert_eq!(state.devices.len(), 1);

    dev.open_ports.insert(port.clone());

    reducer.reduce(&mut state, Action::AddDevice(dev.clone()));

    assert_eq!(state.devices.len(), 1);
    assert_eq!(state.devices[0], dev);
    assert_eq!(state.devices[0].open_ports.len(), 1);
    assert!(state.devices[0].open_ports.contains(&port));
    tear_down(conf_path);
}
