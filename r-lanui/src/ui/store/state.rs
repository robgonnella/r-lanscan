use core::fmt;
use std::collections::HashMap;

use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    layout::Rect,
    style::{palette::tailwind, Color},
};

use crate::config::Config;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ViewID {
    Main,
    Device,
    Devices,
    Config,
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
    pub alt_row_bg: Color,
    pub border_color: Color,
    pub border_focused_color: Color,
    pub scroll_bar_fg: Color,
    pub placeholder: Color,
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
            alt_row_bg: tailwind::SLATE.c900,
            border_color: color.c950,
            border_focused_color: color.c400,
            scroll_bar_fg: tailwind::SLATE.c800,
            placeholder: tailwind::SLATE.c800,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub focused: ViewID,
    pub config: Config,
    pub devices: Vec<DeviceWithPorts>,
    pub device_map: HashMap<String, DeviceWithPorts>,
    pub selected_device: Option<String>,
    pub colors: Colors,
    pub message: Option<String>,
    pub layout: Option<HashMap<ViewID, Rect>>,
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
