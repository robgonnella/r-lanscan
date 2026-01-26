//! Event and command type definitions.

use std::fmt::Display;

use r_lanlib::scanners::Device;

use crate::{config::DeviceConfig, shell::traits::BrowseArgs};

/// External commands that can be executed (SSH, traceroute, browse).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    Ssh(Device, DeviceConfig),
    TraceRoute(Device),
    Browse(BrowseArgs),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Ssh(_, _) => write!(f, "ssh"),
            Command::TraceRoute(_) => write!(f, "traceroute"),
            Command::Browse(_) => write!(f, "browse"),
        }
    }
}

/// UI lifecycle and command events passed between app and event manager.
#[derive(Debug, Eq, PartialEq)]
pub enum Message {
    PauseUI,
    UIPaused,
    ResumeUI,
    UIResumed,
    ExecCommand(Command),
    Quit,
}

#[cfg(test)]
#[path = "./message_tests.rs"]
mod tests;
