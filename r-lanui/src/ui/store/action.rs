use std::collections::HashMap;

use r_lanlib::scanners::DeviceWithPorts;
use ratatui::layout::{Position, Rect};

use crate::config::Config;

use super::state::{Theme, ViewID};

#[derive(Debug)]
pub enum Action<'view, 'conf_id, 'theme, 'devices, 'device, 'selected, 'config> {
    UpdateFocus(&'view ViewID),
    UpdateLayout(Option<HashMap<ViewID, Rect>>),
    UpdateMessage(Option<String>),
    Click(Position),
    UpdateTheme((&'conf_id str, &'theme Theme)),
    UpdateAllDevices(&'devices Vec<DeviceWithPorts>),
    AddDevice(&'device DeviceWithPorts),
    UpdateSelectedDevice(&'selected str),
    SetConfig(&'conf_id str),
    CreateAndSetConfig(&'config Config),
}
