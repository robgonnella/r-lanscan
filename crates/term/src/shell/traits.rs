//! Traits for shell command execution.

#[cfg(test)]
use mockall::automock;

use color_eyre::eyre::Result;
use r_lanlib::scanners::Device;
use std::process::{ChildStderr, ExitStatus, Output};

use crate::config::DeviceConfig;

/// Arguments for opening a web browser on a device port.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BrowseArgs {
    pub device: Device,
    pub port: u16,
    pub use_lynx: bool,
}

/// Trait for executing shell commands. Abstracted for testability.
#[cfg_attr(test, automock)]
pub trait ShellExecutor: Send + Sync {
    /// Opens an SSH session to the given device using the provided config.
    fn ssh(
        &self,
        device: &Device,
        config: &DeviceConfig,
    ) -> Result<(ExitStatus, Option<ChildStderr>)>;

    /// Runs traceroute to the given device and returns the output.
    fn traceroute(&self, device: &Device) -> Result<Output>;

    /// Opens a web browser to the device's port (uses lynx or system browser).
    fn browse(
        &self,
        args: &BrowseArgs,
    ) -> Result<(ExitStatus, Option<ChildStderr>)>;
}
