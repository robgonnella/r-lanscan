use r_lanlib::scanners::DeviceWithPorts;

use crate::config::Config;

use super::types::{Theme, ViewName};

#[derive(Debug)]
pub enum Action {
    UpdateView(ViewName),
    UpdateTheme((String, Theme)),
    UpdateDevices(Vec<DeviceWithPorts>),
    UpdateSelectedDevice(usize),
    SetConfig(String),
    CreateAndSetConfig(Config),
}
