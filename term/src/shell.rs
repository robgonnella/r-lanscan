//! Shell command execution for SSH, traceroute, and browser launch.

use color_eyre::eyre::Result;
use r_lanlib::scanners::Device;
use std::process::{
    ChildStderr, Command as ShellCommand, ExitStatus, Output, Stdio,
};

use crate::{
    config::DeviceConfig,
    shell::traits::{BrowseArgs, ShellExecutor},
};

pub mod traits;

#[cfg(target_os = "linux")]
const fn browser_command() -> &'static str {
    "xdg-open"
}

#[cfg(target_os = "macos")]
const fn browser_command() -> &'static str {
    "open"
}

/// Default implementation of `ShellExecutor` that spawns real shell commands.
#[derive(Default)]
pub struct Shell {}

impl Shell {
    /// Creates a new Shell instance.
    pub fn new() -> Self {
        Self {}
    }
}

impl ShellExecutor for Shell {
    fn ssh(
        &self,
        device: &Device,
        config: &DeviceConfig,
    ) -> Result<(ExitStatus, Option<ChildStderr>)> {
        let mut handle = ShellCommand::new("ssh")
            .arg("-i")
            .arg(config.ssh_identity_file.clone())
            .arg(format!("{}@{}", config.ssh_user, device.ip))
            .arg("-p")
            .arg(config.ssh_port.to_string())
            .stderr(Stdio::piped())
            .spawn()?;

        let status = handle.wait().map_err(Box::new)?;

        Ok((status, handle.stderr))
    }

    fn traceroute(&self, device: &Device) -> Result<Output> {
        let output = ShellCommand::new("traceroute")
            .arg("-w")
            .arg("2")
            .arg("-I")
            .arg("-v")
            .arg("-m")
            .arg("5")
            .arg(device.ip.to_string())
            .output()?;
        Ok(output)
    }

    fn browse(
        &self,
        args: &BrowseArgs,
    ) -> Result<(ExitStatus, Option<ChildStderr>)> {
        let mut protocol = "http";
        if args.port == 443 {
            protocol = "https"
        }
        let url = format!("{}://{}:{}", protocol, args.device.ip, args.port);
        if args.use_lynx {
            let mut handle = ShellCommand::new("lynx")
                .arg(url)
                .stderr(Stdio::piped())
                .env("TERM", "xterm")
                .spawn()?;

            let status = handle.wait().map_err(Box::new)?;
            return Ok((status, handle.stderr));
        }

        let mut handle = ShellCommand::new(browser_command())
            .arg(url)
            .stderr(Stdio::piped())
            .spawn()?;

        let status = handle.wait().map_err(Box::new)?;
        Ok((status, handle.stderr))
    }
}
