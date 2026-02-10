//! Application state definitions.

use std::{
    collections::{HashMap, VecDeque},
    net::Ipv4Addr,
    process::Output,
};

use r_lanlib::scanners::Device;

use crate::{config::Config, ipc::message::Command, ui::colors::Colors};

use crate::ui::colors::Theme;

/// Maximum log lines to store in state
pub const MAX_LOGS: usize = 100;

/// Complete application state for the terminal UI.
#[derive(Debug, Clone)]
pub struct State {
    pub true_color_enabled: bool,
    pub theme: Theme,
    pub ui_paused: bool,
    pub error: Option<String>,
    pub logs: VecDeque<String>,
    pub config: Config,
    pub device_map: HashMap<Ipv4Addr, Device>,
    pub sorted_device_list: Vec<Device>,
    pub colors: Colors,
    pub message: Option<String>,
    pub cmd_in_progress: Option<Command>,
    pub cmd_output: Option<(Command, Output)>,
    pub popover_message: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            true_color_enabled: Default::default(),
            theme: Default::default(),
            ui_paused: Default::default(),
            error: Default::default(),
            logs: VecDeque::with_capacity(MAX_LOGS),
            config: Default::default(),
            device_map: Default::default(),
            sorted_device_list: Default::default(),
            colors: Default::default(),
            message: Default::default(),
            cmd_in_progress: Default::default(),
            cmd_output: Default::default(),
            popover_message: Default::default(),
        }
    }
}

#[cfg(test)]
impl State {
    pub fn default() -> Self {
        let user = "user".to_string();
        let identity = "/home/user/.ssh/id_rsa".to_string();
        let cidr = "192.168.1.1/24".to_string();
        let config = Config::new(user, identity, cidr);
        let theme = Theme::from_string(&config.theme);
        let true_color_enabled = true;
        let colors = crate::ui::colors::Colors::new(
            theme.to_palette(true_color_enabled),
            true_color_enabled,
        );

        Self {
            true_color_enabled,
            theme,
            ui_paused: false,
            error: None,
            logs: VecDeque::with_capacity(MAX_LOGS),
            config,
            device_map: HashMap::new(),
            sorted_device_list: vec![],
            colors,
            message: None,
            cmd_in_progress: None,
            cmd_output: None,
            popover_message: None,
        }
    }
}
