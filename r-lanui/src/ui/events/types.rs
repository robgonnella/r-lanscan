use std::fmt::Display;

use r_lanlib::scanners::Device;

use crate::config::DeviceConfig;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    SSH(Device, DeviceConfig),
    TRACEROUTE(Device),
    BROWSE(Device, u16),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::SSH(_, _) => write!(f, "ssh"),
            Command::TRACEROUTE(_) => write!(f, "traceroute"),
            Command::BROWSE(_, _) => write!(f, "browse"),
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
