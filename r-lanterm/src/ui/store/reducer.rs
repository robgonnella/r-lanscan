use std::{
    collections::HashMap,
    net::Ipv4Addr,
    str::FromStr,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use r_lanlib::scanners::{Device, DeviceWithPorts};

use crate::{
    config::{ConfigManager, DeviceConfig},
    ui::colors::{Colors, Theme},
};

use super::{action::Action, state::State};

const MAX_ARP_MISS: i8 = 3;

pub struct Reducer {
    config_manager: Arc<Mutex<ConfigManager>>,
}

impl Reducer {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self { config_manager }
    }

    pub fn reduce(&self, prev_state: State, action: Action) -> State {
        let new_state = match action {
            Action::SetUIPaused(value) => {
                let mut state = prev_state.clone();
                state.ui_paused = value;
                state
            }
            Action::SetError(err) => {
                let mut state = prev_state.clone();
                state.error = err;
                state
            }
            Action::ToggleViewSelect => {
                let mut state = prev_state.clone();
                state.render_view_select = !state.render_view_select;
                state
            }
            Action::UpdateView(id) => {
                let mut state = prev_state.clone();
                state.view_id = id;
                state
            }
            Action::UpdateMessage(message) => {
                let mut state = prev_state.clone();
                state.message = message;
                state
            }
            Action::PreviewTheme(theme) => {
                let mut state = prev_state.clone();
                state.colors = Colors::new(
                    theme.to_palette(state.true_color_enabled),
                    state.true_color_enabled,
                );
                state
            }
            Action::UpdateConfig(config) => {
                let mut state = prev_state.clone();
                let mut manager = self.config_manager.lock().unwrap();
                manager.update_config(config.clone());
                state.config = config;
                state
            }
            Action::UpdateAllDevices(devices) => {
                let mut state = prev_state.clone();
                let mut new_map: HashMap<String, DeviceWithPorts> = HashMap::new();
                let mut arp_history: HashMap<String, (Device, i8)> = HashMap::new();

                for d in devices.iter() {
                    new_map.insert(d.mac.clone(), d.clone());
                }

                state.devices = devices.clone();
                state
                    .devices
                    .sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());

                state.device_map = new_map;

                // keep devices that may have been missed in last scan but
                // up to a max limit of misses
                for d in state.arp_history.iter() {
                    let mut count = d.1 .1.clone();
                    if !state.device_map.contains_key(d.0) {
                        count += 1;
                    }

                    if count < MAX_ARP_MISS {
                        arp_history.insert(d.0.clone(), (d.1 .0.clone(), count));
                    }
                }

                state.arp_history = arp_history;
                state
            }
            Action::AddDevice(device) => {
                let mut state = prev_state.clone();
                let arp_device: Device = device.clone().into();

                state
                    .arp_history
                    .insert(device.mac.clone(), (arp_device, 0));

                if state.device_map.contains_key(&device.mac.clone()) {
                    let found_device = state
                        .devices
                        .iter_mut()
                        .find(|d| d.mac == device.mac)
                        .unwrap();
                    found_device.hostname = device.hostname.clone();
                    found_device.ip = device.ip.clone();
                    found_device.mac = device.mac.clone();

                    for p in &device.open_ports {
                        found_device.open_ports.insert(p.clone());
                    }

                    found_device.open_ports.iter().sorted_by_key(|p| p.id);
                    let mapped_device = state.device_map.get_mut(&device.mac.clone()).unwrap();
                    *mapped_device = found_device.clone();
                } else {
                    state.devices.push(device.clone());
                    state.device_map.insert(device.mac.clone(), device.clone());
                }

                state
                    .devices
                    .sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());
                state
            }
            Action::SetConfig(config_id) => {
                let mut state = prev_state.clone();
                if let Some(conf) = self
                    .config_manager
                    .lock()
                    .unwrap()
                    .get_by_id(config_id.as_str())
                {
                    let theme = Theme::from_string(&conf.theme);
                    state.config = conf;
                    state.colors = Colors::new(
                        theme.to_palette(state.true_color_enabled),
                        state.true_color_enabled,
                    );
                }
                state
            }
            Action::CreateAndSetConfig(config) => {
                let mut state = prev_state.clone();
                let mut manager = self.config_manager.lock().unwrap();
                manager.create(&config);
                let theme = Theme::from_string(&config.theme);
                state.config = config.clone();
                state.colors = Colors::new(
                    theme.to_palette(state.true_color_enabled),
                    state.true_color_enabled,
                );
                state
            }
            Action::UpdateSelectedDevice(i) => {
                let mut state = prev_state.clone();
                if let Some(device) = state.device_map.get(i.as_str()) {
                    state.selected_device = Some(device.clone());
                    let device_config: DeviceConfig;
                    if state.config.device_configs.contains_key(&device.ip) {
                        device_config =
                            state.config.device_configs.get(&device.ip).unwrap().clone();
                    } else if state.config.device_configs.contains_key(&device.mac) {
                        device_config = state
                            .config
                            .device_configs
                            .get(&device.mac)
                            .unwrap()
                            .clone();
                    } else {
                        device_config = DeviceConfig {
                            id: device.mac.clone(),
                            ssh_identity_file: state.config.default_ssh_identity.clone(),
                            ssh_port: state
                                .config
                                .default_ssh_port
                                .clone()
                                .parse::<u16>()
                                .unwrap(),
                            ssh_user: state.config.default_ssh_user.clone(),
                        }
                    }

                    state.selected_device_config = Some(device_config);
                }

                state
            }
            Action::UpdateDeviceConfig(device_config) => {
                let mut state = prev_state.clone();
                let mut config = state.config.clone();
                config
                    .device_configs
                    .insert(device_config.id.clone(), device_config);
                let mut manager = self.config_manager.lock().unwrap();
                manager.update_config(config.clone());
                state.config = config;
                state
            }
            Action::SetCommandInProgress(value) => {
                let mut state = prev_state.clone();
                state.cmd_in_progress = value;
                state
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                let mut state = prev_state.clone();
                state.cmd_output = Some((cmd, output));
                state
            }
            Action::ClearCommandOutput => {
                let mut state = prev_state.clone();
                state.cmd_output = None;
                state
            }
        };

        new_state
    }
}

#[cfg(test)]
mod tests {
    use nanoid::nanoid;
    use pnet::util::MacAddr;
    use r_lanlib::scanners::Port;
    use std::{collections::HashSet, fs, os::unix::process::ExitStatusExt, process::Output};

    use crate::{
        config::Config,
        ui::{events::types::Command, store::state::ViewID},
    };

    use super::*;

    fn setup() -> (State, Reducer, String) {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));

        let starting_state = State::default();
        let reducer = Reducer::new(conf_manager);

        (starting_state, reducer, tmp_path)
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_ui_paused() {
        let (starting_state, reducer, conf_path) = setup();

        let mut state = reducer.reduce(starting_state.clone(), Action::SetUIPaused(true));
        assert!(state.ui_paused);

        state = reducer.reduce(starting_state.clone(), Action::SetUIPaused(false));
        assert!(!state.ui_paused);
        tear_down(conf_path);
    }

    #[test]
    fn test_set_error() {
        let (starting_state, reducer, conf_path) = setup();

        let mut state = reducer.reduce(
            starting_state.clone(),
            Action::SetError(Some("error".to_string())),
        );
        assert!(state.error.is_some());

        state = reducer.reduce(starting_state.clone(), Action::SetError(None));
        assert!(state.error.is_none());

        tear_down(conf_path);
    }

    #[test]
    fn test_toggle_view_select() {
        let (starting_state, reducer, conf_path) = setup();
        let state = reducer.reduce(starting_state.clone(), Action::ToggleViewSelect);
        assert!(state.render_view_select);
        tear_down(conf_path);
    }

    #[test]
    fn test_update_view() {
        let (starting_state, reducer, conf_path) = setup();
        let state = reducer.reduce(starting_state.clone(), Action::UpdateView(ViewID::Config));
        assert_eq!(state.view_id, ViewID::Config);
        tear_down(conf_path);
    }

    #[test]
    fn test_update_message() {
        let (starting_state, reducer, conf_path) = setup();
        let state = reducer.reduce(
            starting_state.clone(),
            Action::UpdateMessage(Some("message".to_string())),
        );
        assert_eq!(state.message.unwrap(), "message".to_string());
        tear_down(conf_path);
    }

    #[test]
    fn test_preview_theme() {
        let (starting_state, reducer, conf_path) = setup();
        let expected_colors = Colors::new(Theme::Emerald.to_palette(true), true);
        let state = reducer.reduce(starting_state.clone(), Action::PreviewTheme(Theme::Emerald));
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
        let (starting_state, reducer, conf_path) = setup();

        let expected_config = Config {
            cidr: "cidr".to_string(),
            default_ssh_identity: "id_rsa".to_string(),
            default_ssh_port: "2222".to_string(),
            default_ssh_user: "user".to_string(),
            device_configs: HashMap::new(),
            id: "config_id".to_string(),
            ports: vec!["80".to_string(), "443".to_string()],
            theme: "Emerald".to_string(),
        };

        let state = reducer.reduce(
            starting_state.clone(),
            Action::UpdateConfig(expected_config.clone()),
        );
        assert_eq!(state.config, expected_config);

        tear_down(conf_path);
    }

    #[test]
    fn test_update_all_devices() {
        let (starting_state, reducer, conf_path) = setup();

        let dev1 = DeviceWithPorts {
            hostname: "dev1".to_string(),
            ip: "10.10.10.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            open_ports: HashSet::new(),
            vendor: "dev1_vendor".to_string(),
        };

        let dev2 = DeviceWithPorts {
            hostname: "dev2".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: false,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            open_ports: HashSet::new(),
            vendor: "dev2_vendor".to_string(),
        };

        let expected_devices = vec![dev1.clone(), dev2.clone()];

        let state = reducer.reduce(
            starting_state.clone(),
            Action::UpdateAllDevices(expected_devices.clone()),
        );
        assert_eq!(state.devices, expected_devices);

        tear_down(conf_path);
    }

    #[test]
    fn test_add_device() {
        let (starting_state, reducer, conf_path) = setup();

        let dev3 = DeviceWithPorts {
            hostname: "dev3".to_string(),
            ip: "dev3_ip".to_string(),
            is_current_host: false,
            mac: "dev3_mac".to_string(),
            open_ports: HashSet::new(),
            vendor: "dev3_vendor".to_string(),
        };

        let state = reducer.reduce(starting_state.clone(), Action::AddDevice(dev3.clone()));
        assert_eq!(state.devices, vec![dev3.clone()]);
        tear_down(conf_path);
    }

    #[test]
    fn test_set_config() {
        let (starting_state, reducer, conf_path) = setup();
        let state = reducer.reduce(
            starting_state.clone(),
            Action::SetConfig("default".to_string()),
        );
        assert_eq!(state.config.id, "default");
        tear_down(conf_path);
    }

    #[test]
    fn test_create_and_set_config() {
        let (starting_state, reducer, conf_path) = setup();
        let mut config = Config::default();
        config.id = "config_id".to_string();
        let state = reducer.reduce(
            starting_state.clone(),
            Action::CreateAndSetConfig(config.clone()),
        );
        assert_eq!(state.config.id, config.id);
        tear_down(conf_path);
    }

    #[test]
    fn test_update_selected_device() {
        let (starting_state, reducer, conf_path) = setup();

        let dev1 = DeviceWithPorts {
            hostname: "dev1".to_string(),
            ip: "10.10.10.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            open_ports: HashSet::new(),
            vendor: "dev1_vendor".to_string(),
        };

        let dev2 = DeviceWithPorts {
            hostname: "dev2".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: false,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            open_ports: HashSet::new(),
            vendor: "dev2_vendor".to_string(),
        };

        let mut state = reducer.reduce(starting_state, Action::AddDevice(dev1.clone()));
        state = reducer.reduce(state, Action::AddDevice(dev2.clone()));
        state = reducer.reduce(
            state,
            Action::UpdateAllDevices(vec![dev1.clone(), dev2.clone()]),
        );
        state = reducer.reduce(state, Action::UpdateSelectedDevice(dev2.mac.clone()));
        assert!(state.selected_device.is_some());
        let selected = state.selected_device.unwrap();
        assert_eq!(selected.mac, dev2.mac);
        tear_down(conf_path);
    }

    #[test]
    fn test_update_device_config() {
        let (starting_state, reducer, conf_path) = setup();

        let dev = DeviceWithPorts {
            hostname: "dev".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: false,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            open_ports: HashSet::new(),
            vendor: "dev_vendor".to_string(),
        };

        let dev_config = DeviceConfig {
            id: dev.mac.clone(),
            ssh_identity_file: "id_rsa".to_string(),
            ssh_port: 2222,
            ssh_user: "dev_user".to_string(),
        };

        let mut state = reducer.reduce(starting_state, Action::AddDevice(dev.clone()));
        state = reducer.reduce(state, Action::UpdateAllDevices(vec![dev.clone()]));
        state = reducer.reduce(state, Action::UpdateDeviceConfig(dev_config.clone()));
        state = reducer.reduce(state, Action::UpdateSelectedDevice(dev.mac.clone()));

        assert!(state.selected_device_config.is_some());
        let selected = state.selected_device_config.unwrap();
        assert_eq!(selected.id, dev_config.id);
        assert_eq!(selected.ssh_port, dev_config.ssh_port);

        tear_down(conf_path);
    }

    #[test]
    fn test_set_command_in_progress() {
        let (starting_state, reducer, conf_path) = setup();
        let dev = Device {
            hostname: "dev".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: false,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            vendor: "dev_vendor".to_string(),
        };
        let port: u16 = 80;
        let state = reducer.reduce(
            starting_state,
            Action::SetCommandInProgress(Some(Command::BROWSE(dev.clone(), port))),
        );
        assert!(state.cmd_in_progress.is_some());
        let cmd = state.cmd_in_progress.unwrap();
        assert_eq!(cmd, Command::BROWSE(dev, port));
        tear_down(conf_path);
    }

    #[test]
    fn test_update_command_output() {
        let (starting_state, reducer, conf_path) = setup();
        let dev = Device {
            hostname: "dev".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: false,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            vendor: "dev_vendor".to_string(),
        };
        let port: u16 = 80;
        let cmd = Command::BROWSE(dev.clone(), port);

        let output = Output {
            status: ExitStatusExt::from_raw(0),
            stdout: "this is some output".as_bytes().to_vec(),
            stderr: vec![],
        };

        let mut state = reducer.reduce(
            starting_state,
            Action::UpdateCommandOutput((cmd.clone(), output.clone())),
        );
        assert!(state.cmd_output.is_some());
        let info = state.cmd_output.clone().unwrap();
        assert_eq!(info.0, cmd);
        assert_eq!(info.1, output);

        state = reducer.reduce(state, Action::ClearCommandOutput);
        assert!(state.cmd_output.is_none());
        tear_down(conf_path);
    }

    #[test]
    fn test_updates_device_with_new_info() {
        let (starting_state, reducer, conf_path) = setup();

        let mut dev = DeviceWithPorts {
            hostname: "dev".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: false,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            vendor: "dev_vendor".to_string(),
            open_ports: HashSet::new(),
        };

        let port = Port {
            id: 80,
            service: "http".to_string(),
        };

        let mut state = reducer.reduce(starting_state, Action::AddDevice(dev.clone()));

        assert_eq!(state.devices.len(), 1);

        dev.open_ports.insert(port.clone());

        state = reducer.reduce(state, Action::AddDevice(dev.clone()));

        assert_eq!(state.devices.len(), 1);
        assert_eq!(state.devices[0], dev);
        assert_eq!(state.devices[0].open_ports.len(), 1);
        assert!(state.devices[0].open_ports.get(&port).is_some());
        tear_down(conf_path);
    }
}
