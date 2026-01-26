//! Application state definitions.

use core::fmt;
use std::{collections::HashMap, net::Ipv4Addr, process::Output};

use r_lanlib::scanners::Device;

use crate::{
    config::{Config, DeviceConfig},
    ipc::message::Command,
    ui::colors::Colors,
};

#[cfg(test)]
use crate::ui::colors::Theme;

/// Tracks how many scans a device has been missing from.
pub type MissedCount = i8;

/// Identifies the currently active view.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum ViewID {
    Device,
    Devices,
    Config,
    ViewSelect,
}

impl fmt::Display for ViewID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Complete application state for the terminal UI.
#[derive(Debug, Clone)]
pub struct State {
    pub true_color_enabled: bool,
    pub ui_paused: bool,
    pub error: Option<String>,
    pub render_view_select: bool,
    pub view_id: ViewID,
    pub config: Config,
    pub arp_history: HashMap<Ipv4Addr, (Device, MissedCount)>,
    pub device_map: HashMap<Ipv4Addr, Device>,
    pub sorted_device_list: Vec<Device>,
    pub selected_device: Option<Device>,
    pub selected_device_config: Option<DeviceConfig>,
    pub colors: Colors,
    pub message: Option<String>,
    pub cmd_in_progress: Option<Command>,
    pub cmd_output: Option<(Command, Output)>,
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
            ui_paused: false,
            error: None,
            render_view_select: false,
            view_id: ViewID::Devices,
            config,
            arp_history: HashMap::new(),
            device_map: HashMap::new(),
            sorted_device_list: vec![],
            selected_device: None,
            selected_device_config: None,
            colors,
            message: None,
            cmd_in_progress: None,
            cmd_output: None,
        }
    }
}
