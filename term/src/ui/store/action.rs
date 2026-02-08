//! Action types for state transitions.

use std::{collections::HashMap, net::Ipv4Addr, process::Output};

use r_lanlib::scanners::Device;

use crate::{
    config::{Config, DeviceConfig},
    ipc::message::Command,
    ui::colors::Theme,
};

/// Commands that trigger state changes via the reducer.
#[derive(Debug)]
pub enum Action {
    SetUIPaused(bool),
    SetError(Option<String>),
    Log(String),
    SetCommandInProgress(Option<Command>),
    UpdateCommandOutput((Command, Output)),
    ClearCommandOutput,
    UpdateMessage(Option<String>),
    PreviewTheme(Theme),
    UpdateAllDevices(HashMap<Ipv4Addr, Device>),
    AddDevice(Device),
    UpdateConfig(Config),
    UpdateDeviceConfig(DeviceConfig),
    CreateAndSetConfig(Config),
}
