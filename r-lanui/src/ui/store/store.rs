use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use r_lanlib::scanners::DeviceWithPorts;

use crate::config::{Config, ConfigManager, DEFAULT_CONFIG_ID};

use super::{
    action::Action,
    types::{Theme, ViewName},
};

#[derive(Clone)]
pub struct Settings {
    pub theme: Theme,
}

#[derive(Clone)]
pub struct State {
    pub view: ViewName,
    pub config: Config,
    pub devices: Vec<DeviceWithPorts>,
    pub selected_device: usize,
}

pub struct Store {
    config_manager: Arc<Mutex<ConfigManager>>,
    state: State,
}

impl Store {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        let config = config_manager
            .lock()
            .unwrap()
            .get_by_id(DEFAULT_CONFIG_ID.to_string())
            .unwrap();

        Self {
            config_manager,
            state: State {
                view: ViewName::Devices,
                config,
                devices: vec![DeviceWithPorts {
                    hostname: "Scanning…".to_string(),
                    ip: "".to_string(),
                    mac: "".to_string(),
                    vendor: "".to_string(),
                    open_ports: HashSet::new(),
                }],
                selected_device: 0,
            },
        }
    }

    pub fn get_state(&self) -> State {
        self.state.clone()
    }

    pub fn update(&mut self, action: Action) {
        let new_state = match action {
            Action::UpdateView(view) => {
                let mut state = self.state.clone();
                state.view = view;
                state
            }
            Action::UpdateTheme((config_id, theme)) => {
                let mut manager = self.config_manager.lock().unwrap();
                manager.update_theme(config_id.clone(), theme);
                let mut state = self.state.clone();
                state.config = manager.get_by_id(config_id).unwrap();
                state
            }
            Action::UpdateDevices(devices) => {
                let mut state = self.state.clone();
                state.devices = devices;
                state
            }
            Action::SetConfig(config_id) => {
                let mut state = self.state.clone();
                if let Some(conf) = self.config_manager.lock().unwrap().get_by_id(config_id) {
                    state.config = conf;
                }
                state
            }
            Action::CreateAndSetConfig(config) => {
                let mut state = self.state.clone();
                let mut manager = self.config_manager.lock().unwrap();
                manager.create(config.clone());
                state.config = config;
                state
            }
            Action::UpdateSelectedDevice(i) => {
                let mut state = self.state.clone();
                state.selected_device = i;
                state
            }
        };

        self.state = new_state;
    }
}
