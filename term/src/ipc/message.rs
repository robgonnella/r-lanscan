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

/// Messages sent from the renderer to the main event handler.
#[derive(Debug, PartialEq)]
pub enum MainMessage {
    /// UI has been paused (ready for shell command).
    UIPaused,
    /// UI has resumed after shell command.
    UIResumed,
    /// Request to execute an external command.
    ExecCommand(Command),
    /// Request to quit the application.
    Quit,
}

/// Messages sent from the main event handler to the renderer.
#[derive(Debug, PartialEq)]
pub enum RendererMessage {
    /// Request the renderer to pause (exit raw mode for shell command).
    PauseUI,
    /// Request the renderer to resume after shell command.
    ResumeUI,
    /// Instructs renderer process that it needs to redraw
    ReRender,
}

#[cfg(test)]
#[path = "./message_tests.rs"]
mod tests;
