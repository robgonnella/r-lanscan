use std::{
    collections::HashMap,
    net::Ipv4Addr,
    str::FromStr,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;

use crate::config::{ConfigManager, DeviceConfig};

use super::{
    action::Action,
    state::{Colors, State, Theme},
};

pub struct Reducer {
    config_manager: Arc<Mutex<ConfigManager>>,
}

impl Reducer {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self { config_manager }
    }

    pub fn reduce(&self, prev_state: State, action: Action) -> State {
        let new_state = match action {
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
                state.colors = Colors::new(theme.to_palette());
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
                for d in devices.iter() {
                    new_map.insert(d.mac.clone(), d.clone());
                }
                state.devices = devices.clone();
                state
                    .devices
                    .sort_by_key(|i| Ipv4Addr::from_str(&i.ip.to_owned()).unwrap());
                state.device_map = new_map;
                state
            }
            Action::AddDevice(device) => {
                let mut state = prev_state.clone();
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
                    state.colors = Colors::new(theme.to_palette());
                }
                state
            }
            Action::CreateAndSetConfig(config) => {
                let mut state = prev_state.clone();
                let mut manager = self.config_manager.lock().unwrap();
                manager.create(&config);
                let theme = Theme::from_string(&config.theme);
                state.config = config.clone();
                state.colors = Colors::new(theme.to_palette());
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
            Action::ExecuteCommand(cmd) => {
                let mut state = prev_state.clone();
                state.execute_cmd = Some(cmd);
                state.cmd_output = None;
                state
            }
            Action::ClearCommand => {
                let mut state = prev_state.clone();
                state.execute_cmd = None;
                state.cmd_output = None;
                state
            }
            Action::SetCommandInProgress(value) => {
                let mut state = prev_state.clone();
                state.cmd_in_progress = value;
                state
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                let mut state = prev_state.clone();
                state.execute_cmd = None;
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
