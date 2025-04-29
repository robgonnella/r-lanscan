use std::process::Output;

use r_lanlib::scanners::DeviceWithPorts;

use crate::{
    config::{Config, DeviceConfig},
    ui::{colors::Theme, events::types::Command},
};

use super::state::ViewID;

#[derive(Debug)]
pub enum Action {
    SetUIPaused(bool),
    SetError(Option<String>),
    SetCommandInProgress(Option<Command>),
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
