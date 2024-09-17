use r_lanlib::scanners::DeviceWithPorts;

use crate::config::Config;

use super::types::{Theme, ViewName};

#[derive(Debug)]
pub enum Action<'view, 'conf_id, 'theme, 'devices, 'device, 'selected, 'config> {
    UpdateView(&'view ViewName),
    UpdateTheme((&'conf_id String, &'theme Theme)),
    UpdateAllDevices(&'devices Vec<DeviceWithPorts>),
    AddDevice(&'device DeviceWithPorts),
    UpdateSelectedDevice(&'selected String),
    SetConfig(&'conf_id String),
    CreateAndSetConfig(&'config Config),
}
