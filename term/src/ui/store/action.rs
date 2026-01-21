use std::{collections::HashMap, net::Ipv4Addr, process::Output};

use r_lanlib::scanners::Device;

use crate::{
    config::{Config, DeviceConfig},
    events::types::Command,
    ui::colors::Theme,
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
    UpdateAllDevices(HashMap<Ipv4Addr, Device>),
    AddDevice(Device),
    UpdateSelectedDevice(Ipv4Addr),
    UpdateConfig(Config),
    UpdateDeviceConfig(DeviceConfig),
    SetConfig(String),
    CreateAndSetConfig(Config),
}
