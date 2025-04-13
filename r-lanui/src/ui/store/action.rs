use r_lanlib::scanners::DeviceWithPorts;

use crate::config::Config;

use super::state::{Theme, ViewID};

#[derive(Debug)]
pub enum Action<'conf_id, 'theme, 'devices, 'device, 'selected, 'config> {
    ToggleViewSelect,
    TogglePause,
    UpdateView(ViewID),
    UpdateMessage(Option<String>),
    UpdateTheme((&'conf_id str, &'theme Theme)),
    UpdateAllDevices(&'devices Vec<DeviceWithPorts>),
    AddDevice(&'device DeviceWithPorts),
    UpdateSelectedDevice(&'selected str),
    SetConfig(&'conf_id str),
    CreateAndSetConfig(&'config Config),
}
