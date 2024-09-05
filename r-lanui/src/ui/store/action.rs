use r_lanlib::scanners::DeviceWithPorts;

use crate::config::Config;

use super::types::{Theme, ViewName};

#[derive(Debug)]
pub enum Action<'view, 'conf_id, 'theme, 'devices, 'selected, 'config> {
    UpdateView(&'view ViewName),
    UpdateTheme((&'conf_id String, &'theme Theme)),
    UpdateDevices(&'devices Vec<DeviceWithPorts>),
    UpdateSelectedDevice(&'selected usize),
    SetConfig(&'conf_id String),
    CreateAndSetConfig(&'config Config),
}
