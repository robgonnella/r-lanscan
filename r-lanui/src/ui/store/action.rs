use r_lanlib::scanners::DeviceWithPorts;

use crate::config::{Config, DeviceConfig};

use super::state::{Theme, ViewID};

#[derive(Debug)]
pub enum Action {
    ToggleViewSelect,
    TogglePause,
    UpdateView(ViewID),
    UpdateMessage(Option<String>),
    PreviewTheme(Theme),
    UpdateAllDevices(Vec<DeviceWithPorts>),
    AddDevice(DeviceWithPorts),
    UpdateSelectedDevice(String),
    UpdateConfig(Config),
    UpdateDeviceConfig(DeviceConfig),
    SetConfig(String),
    CreateAndSetConfig(Config),
}
