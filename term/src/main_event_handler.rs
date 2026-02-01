//! Processes UI events and executes external commands.

use color_eyre::eyre::Result;
use r_lanlib::scanners::Device;
use std::{
    io::{BufReader, Read},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    config::DeviceConfig,
    ipc::{
        main::MainIpc,
        message::{Command as AppCommand, MainMessage, RendererMessage},
    },
    shell::traits::{BrowseArgs, ShellExecutor},
    ui::store::{Dispatcher, StateGetter, Store, action::Action},
};

/// Runs the event loop, handling UI pause/resume and command execution.
pub struct MainEventHandler {
    store: Arc<Store>,
    executor: Box<dyn ShellExecutor>,
    ipc: MainIpc,
}

impl MainEventHandler {
    /// Creates a new event handler with the given store, shell executor, and
    /// IPC channels.
    pub fn new(
        store: Arc<Store>,
        executor: Box<dyn ShellExecutor>,
        ipc: MainIpc,
    ) -> Self {
        Self {
            store,
            executor,
            ipc,
        }
    }

    /// Runs the main event loop, blocking until a quit message is received.
    /// Dispatches commands to the appropriate handler (SSH, traceroute, browse).
    pub fn process_events(&self) -> Result<()> {
        loop {
            if let Ok(evt) = self.ipc.rx.recv() {
                // event loop
                match evt {
                    MainMessage::ExecCommand(cmd) => {
                        let _ = self.handle_cmd(cmd);
                    }
                    MainMessage::Quit => return Ok(()),
                    _ => {}
                }
            }
        }
    }

    fn pause_ui(&self) -> Result<()> {
        // send event to app thread to pause UI
        self.ipc.tx.send(RendererMessage::PauseUI)?;
        let start = Instant::now();
        let timeout = Duration::from_millis(5000);
        // wait for app to respond that UI has been paused
        loop {
            // continue if we're stuck waiting for UI to respond
            if start.elapsed() >= timeout {
                return Ok(());
            }
            if let Ok(evt) = self.ipc.rx.recv()
                && evt == MainMessage::UIPaused
            {
                return Ok(());
            }
        }
    }

    fn handle_ssh(
        &self,
        device: &Device,
        device_config: &DeviceConfig,
    ) -> Result<()> {
        self.pause_ui()?;
        let res = self.executor.ssh(device, device_config);
        self.ipc.tx.send(RendererMessage::ResumeUI)?;
        match res {
            Ok((status, err)) => {
                if !status.success() {
                    if let Some(stderr) = err {
                        let mut stderr_output = String::new();
                        let mut stderr_reader = BufReader::new(stderr);
                        stderr_reader.read_to_string(&mut stderr_output)?;
                        self.store
                            .dispatch(Action::SetError(Some(stderr_output)))?;
                    } else {
                        let err = String::from("ssh command failed");
                        self.store.dispatch(Action::SetError(Some(err)))?;
                    }
                }
            }
            Err(e) => {
                self.store.dispatch(Action::SetError(Some(e.to_string())))?;
            }
        }

        Ok(())
    }

    fn handle_traceroute(
        &self,
        cmd: &AppCommand,
        device: &Device,
    ) -> Result<()> {
        let exec = self.executor.traceroute(device);
        match exec {
            Ok(output) => {
                self.store.dispatch(Action::UpdateCommandOutput((
                    cmd.clone(),
                    output,
                )))?;
                self.store.dispatch(Action::SetCommandInProgress(None))?;
            }
            Err(err) => {
                self.store
                    .dispatch(Action::SetError(Some(err.to_string())))?;
            }
        }

        Ok(())
    }

    fn handle_browse(&self, args: &BrowseArgs) -> Result<()> {
        self.pause_ui()?;

        let res = self.executor.browse(args);

        self.ipc.tx.send(RendererMessage::ResumeUI)?;

        match res {
            Ok((status, err)) => {
                if !status.success() {
                    if let Some(stderr) = err {
                        let mut stderr_output = String::new();
                        let mut stderr_reader = BufReader::new(stderr);
                        stderr_reader.read_to_string(&mut stderr_output)?;
                        self.store
                            .dispatch(Action::SetError(Some(stderr_output)))?;
                    } else {
                        let err = String::from("lynx command failed");
                        self.store.dispatch(Action::SetError(Some(err)))?;
                    }
                }
            }
            Err(e) => {
                self.store.dispatch(Action::SetError(Some(e.to_string())))?;
            }
        }

        Ok(())
    }

    fn handle_cmd(&self, cmd: AppCommand) -> Result<()> {
        let state = self.store.get_state()?;

        if state.cmd_in_progress.is_some() {
            return Ok(());
        }

        self.store
            .dispatch(Action::SetCommandInProgress(Some(cmd.clone())))?;

        match &cmd {
            AppCommand::Ssh(device, device_config) => {
                self.handle_ssh(device, device_config)?
            }
            AppCommand::TraceRoute(device) => {
                self.handle_traceroute(&cmd, device)?
            }
            AppCommand::Browse(args) => self.handle_browse(args)?,
        }

        self.store.dispatch(Action::SetCommandInProgress(None))?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "./main_event_handler_tests.rs"]
mod tests;
