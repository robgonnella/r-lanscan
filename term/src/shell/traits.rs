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

/// Trait for executing shell commands making it easier to mock in tests
#[cfg_attr(test, automock)]
pub trait ShellExecutor: Send {
    fn ssh(
        &self,
        device: &Device,
        config: &DeviceConfig,
    ) -> Result<(ExitStatus, Option<ChildStderr>)>;

    fn traceroute(&self, device: &Device) -> Result<Output>;

    fn browse(&self, args: &BrowseArgs) -> Result<(ExitStatus, Option<ChildStderr>)>;
}
