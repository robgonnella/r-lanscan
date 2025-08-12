use std::fmt::Display;

use r_lanlib::scanners::Device;

use crate::config::DeviceConfig;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BrowseArgs {
    pub device: Device,
    pub port: u16,
    pub use_lynx: bool,
}

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

#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    PauseUI,
    UIPaused,
    ResumeUI,
    UIResumed,
    ExecCommand(Command),
    Quit,
}

#[cfg(test)]
#[path = "./types_tests.rs"]
mod tests;
