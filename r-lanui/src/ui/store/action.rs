use std::process::Output;

use r_lanlib::scanners::DeviceWithPorts;

use crate::config::{Config, DeviceConfig};

use super::state::{Command, Theme, ViewID};

#[derive(Debug)]
pub enum Action {
    SetError(Option<String>),
    ClearCommand,
    ExecuteCommand(Command),
    SetCommandInProgress(bool),
    UpdateCommandOutput((Command, Output)),
    ClearCommandOutput,
    ToggleViewSelect,
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
