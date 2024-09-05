use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use r_lanlib::scanners::DeviceWithPorts;
use ratatui::style::{palette::tailwind, Color};

use crate::config::{Config, ConfigManager, DEFAULT_CONFIG_ID};

use super::{
    action::Action,
    types::{Theme, ViewName},
};

#[derive(Clone, Debug)]
pub struct Colors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub selected_style_fg: Color,
    pub normal_row_color: Color,
    pub alt_row_color: Color,
    pub footer_border_color: Color,
}

impl Colors {
    pub fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub view: ViewName,
    pub config: Config,
    pub devices: Vec<DeviceWithPorts>,
    pub selected_device: usize,
    pub colors: Colors,
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
            .get_by_id(&DEFAULT_CONFIG_ID.to_string())
            .unwrap();

        let theme = Theme::from_string(&config.theme);
        let colors = Colors::new(theme.to_palette());

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
                colors,
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
                state.view = view.clone();
                state
            }
            Action::UpdateTheme((config_id, theme)) => {
                let mut manager = self.config_manager.lock().unwrap();
                manager.update_theme(config_id, theme);
                let mut state = self.state.clone();
                state.config = manager.get_by_id(config_id).unwrap();
                state.colors = Colors::new(theme.to_palette());
                state
            }
            Action::UpdateDevices(devices) => {
                let mut state = self.state.clone();
                state.devices = devices.clone();
                state
            }
            Action::SetConfig(config_id) => {
                let mut state = self.state.clone();
                if let Some(conf) = self.config_manager.lock().unwrap().get_by_id(config_id) {
                    let theme = Theme::from_string(&conf.theme);
                    state.config = conf;
                    state.colors = Colors::new(theme.to_palette());
                }
                state
            }
            Action::CreateAndSetConfig(config) => {
                let mut state = self.state.clone();
                let mut manager = self.config_manager.lock().unwrap();
                manager.create(config);
                let theme = Theme::from_string(&config.theme);
                state.config = config.clone();
                state.colors = Colors::new(theme.to_palette());
                state
            }
            Action::UpdateSelectedDevice(i) => {
                let mut state = self.state.clone();
                state.selected_device = *i;
                state
            }
        };

        self.state = new_state;
    }
}
