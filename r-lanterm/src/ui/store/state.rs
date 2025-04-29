use core::fmt;
use std::{collections::HashMap, process::Output};

use r_lanlib::scanners::{Device, DeviceWithPorts};

use crate::{
    config::{Config, DeviceConfig},
    ui::{colors::Colors, events::types::Command},
};

pub type MissedCount = i8;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ViewID {
    Main,
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

#[derive(Debug, Clone)]
pub struct State {
    pub true_color_enabled: bool,
    pub ui_paused: bool,
    pub error: Option<String>,
    pub render_view_select: bool,
    pub view_id: ViewID,
    pub config: Config,
    pub arp_history: HashMap<String, (Device, MissedCount)>,
    pub devices: Vec<DeviceWithPorts>,
    pub device_map: HashMap<String, DeviceWithPorts>,
    pub selected_device: Option<DeviceWithPorts>,
    pub selected_device_config: Option<DeviceConfig>,
    pub colors: Colors,
    pub message: Option<String>,
    pub cmd_in_progress: Option<Command>,
    pub cmd_output: Option<(Command, Output)>,
}
