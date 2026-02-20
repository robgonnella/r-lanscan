//! Action types for state transitions.

use std::process::Output;

use r_lanlib::scanners::Device;

use crate::{
    config::{Config, DeviceConfig},
    ipc::message::Command,
    ui::colors::Theme,
};

/// Commands that trigger state changes via the reducer.
#[derive(Debug, Clone)]
pub enum Action {
    SetUIPaused(bool),
    SetError(Option<String>),
    Log(String),
    SetCommandInProgress(Option<Command>),
    UpdateCommandOutput((Command, Output)),
    ClearCommandOutput,
    UpdateMessage(Option<String>),
    PreviewTheme(Theme),
    AddDevice(Device),
    UpdateDevicePorts(Device),
    UpdateConfig(Config),
    RemoveDeviceConfig(String),
    UpdateDeviceConfig(DeviceConfig),
    Sync(Box<Action>),
}
