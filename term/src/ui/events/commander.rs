#[cfg(test)]
use mockall::automock;

use r_lanlib::scanners::Device;
use std::{
    error::Error,
    process::{ChildStderr, Command as ShellCommand, ExitStatus, Output, Stdio},
};

use crate::config::DeviceConfig;

pub struct Commander {}

// generates mocked implementation of Commander when in test
#[cfg_attr(test, automock, allow(warnings))]
impl Commander {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ssh(
        &self,
        device: Device,
        config: DeviceConfig,
    ) -> Result<(ExitStatus, Option<ChildStderr>), Box<dyn Error>> {
        let mut handle = ShellCommand::new("ssh")
            .arg("-i")
            .arg(config.ssh_identity_file)
            .arg(format!("{}@{}", config.ssh_user, device.ip))
            .arg("-p")
            .arg(config.ssh_port.to_string())
            .stderr(Stdio::piped())
            .spawn()?;

        let status = handle.wait().map_err(Box::new)?;

        Ok((status, handle.stderr))
    }

    pub fn traceroute(&self, device: Device) -> Result<Output, Box<dyn Error>> {
        ShellCommand::new("traceroute")
            .arg("-w")
            .arg("2")
            .arg("-I")
            .arg("-v")
            .arg(device.ip)
            .output()
            .map_err(|e| Box::from(e.to_string()))
    }

    pub fn lynx(
        &self,
        device: Device,
        port: u16,
    ) -> Result<(ExitStatus, Option<ChildStderr>), Box<dyn Error>> {
        let mut handle = ShellCommand::new("lynx")
            .arg(format!("{}:{}", device.ip, port))
            .stderr(Stdio::piped())
            .env("TERM", "xterm")
            .spawn()?;

        let status = handle.wait().map_err(Box::new)?;

        Ok((status, handle.stderr))
    }
}
