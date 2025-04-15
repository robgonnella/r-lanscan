use core::fmt;
use std::{collections::HashMap, fmt::Display, process::Output};

use r_lanlib::scanners::DeviceWithPorts;
use ratatui::style::{palette::tailwind, Color};

use crate::config::{Config, DeviceConfig};

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

#[derive(Clone, Debug)]
pub struct Colors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub selected_row_fg: Color,
    pub row_fg: Color,
    pub row_bg: Color,
    pub border_color: Color,
    pub scroll_bar_fg: Color,
    pub label: Color,
}

impl Colors {
    pub fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            selected_row_fg: color.c400,
            row_fg: tailwind::SLATE.c200,
            row_bg: tailwind::SLATE.c950,
            border_color: color.c400,
            scroll_bar_fg: tailwind::SLATE.c800,
            label: color.c400,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    SSH,
    TRACEROUTE,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::SSH => write!(f, "ssh"),
            Command::TRACEROUTE => write!(f, "traceroute"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub render_view_select: bool,
    pub view_id: ViewID,
    pub config: Config,
    pub devices: Vec<DeviceWithPorts>,
    pub device_map: HashMap<String, DeviceWithPorts>,
    pub selected_device: Option<DeviceWithPorts>,
    pub selected_device_config: Option<DeviceConfig>,
    pub colors: Colors,
    pub message: Option<String>,
    pub execute_cmd: Option<Command>,
    pub cmd_output: Option<(Command, Output)>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Theme {
    Blue,
    Emerald,
    Indigo,
    Red,
}

impl Theme {
    pub fn from_string(value: &String) -> Theme {
        match value.as_str() {
            "Blue" => Theme::Blue,
            "Emerald" => Theme::Emerald,
            "Indigo" => Theme::Indigo,
            "Red" => Theme::Red,
            _ => Theme::Blue,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Theme::Blue => "Blue".to_string(),
            Theme::Emerald => "Emerald".to_string(),
            Theme::Indigo => "Indigo".to_string(),
            Theme::Red => "Red".to_string(),
        }
    }

    pub fn to_palette(&self) -> &'static tailwind::Palette {
        match self {
            Theme::Blue => &tailwind::BLUE,
            Theme::Emerald => &tailwind::EMERALD,
            Theme::Indigo => &tailwind::INDIGO,
            Theme::Red => &tailwind::RED,
        }
    }
}
