//! Event and command type definitions.

use r_lanlib::scanners::Device;
use std::fmt::Display;

use crate::{
    config::{Config, DeviceConfig},
    shell::traits::BrowseArgs,
    store::action::Action,
};

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

#[derive(Debug)]
pub enum NetworkMessage {
    /// Instructs network thread to quit
    Quit,
    /// Informs network thread that user updated config
    ConfigUpdate(Config),
}

/// Messages sent from the renderer to the main event handler.
#[derive(Debug)]
pub enum MainMessage {
    /// State updates from the renderer thread
    ActionSync(Box<Action>),
    /// UI has been paused (ready for shell command).
    UIPaused,
    /// UI has resumed after shell command.
    UIResumed,
    /// Informs that ARP scanning is beginning
    ArpStart,
    /// Network ARP update
    ArpUpdate(Device),
    /// Informs that ARP scanning finished
    ArpDone,
    /// Informs that SYN scanning is beginning
    SynStart,
    /// Network SYN update
    SynUpdate(Device),
    /// Informs that SYN scanning finished
    SynDone,
    /// Request to execute an external command.
    ExecCommand(Command),
    /// Request to quit the application.
    Quit(Option<String>),
}

/// Messages sent from the main event handler to the renderer.
#[derive(Debug)]
pub enum RendererMessage {
    /// State updates from the main thread
    ActionSync(Box<Action>),
    /// Request the renderer to pause (exit raw mode for shell command).
    PauseUI,
    /// Request the renderer to resume after shell command.
    ResumeUI,
}

#[cfg(test)]
#[path = "./message_tests.rs"]
mod tests;
