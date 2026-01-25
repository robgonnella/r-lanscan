//! Processes UI events and executes external commands.

use color_eyre::eyre::Result;
use mockall_double::double;
use r_lanlib::scanners::Device;
use std::{
    io::{BufReader, Read},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use crate::{
    config::DeviceConfig,
    events::types::BrowseArgs,
    ui::store::{Dispatcher, Store, action::Action},
};

use super::types::{Command as AppCommand, Event};

// double allows tests to use the mocked version of Commander
#[double]
use super::commander::Commander;

/// Runs the event loop, handling UI pause/resume and command execution.
pub struct EventManager {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    store: Arc<Store>,
    commander: Commander,
}

impl EventManager {
    pub fn new(tx: Sender<Event>, rx: Receiver<Event>, store: Arc<Store>) -> Self {
        Self {
            tx,
            rx,
            store,
            commander: Commander::new(),
        }
    }

    pub fn start_event_loop(&self) -> Result<()> {
        loop {
            if let Ok(evt) = self.rx.recv() {
                // event loop
                match evt {
                    Event::ExecCommand(cmd) => {
                        let _ = self.handle_cmd(cmd);
                    }
                    Event::Quit => return Ok(()),
                    _ => {}
                }
            }
        }
    }

    fn pause_ui(&self) -> Result<()> {
        // send event to app thread to pause UI
        self.tx.send(Event::PauseUI)?;
        // wait for app to respond that UI has been paused
        loop {
            if let Ok(evt) = self.rx.recv()
                && evt == Event::UIPaused
            {
                return Ok(());
            }
        }
    }

    fn handle_ssh(&self, device: &Device, device_config: &DeviceConfig) -> Result<()> {
        self.pause_ui()?;
        let res = self.commander.ssh(device, device_config);
        self.tx.send(Event::ResumeUI)?;
        match res {
            Ok((status, err)) => {
                if !status.success() {
                    if let Some(stderr) = err {
                        let mut stderr_output = String::new();
                        let mut stderr_reader = BufReader::new(stderr);
                        stderr_reader.read_to_string(&mut stderr_output)?;
                        self.store.dispatch(Action::SetError(Some(stderr_output)));
                    } else {
                        let err = String::from("ssh command failed");
                        self.store.dispatch(Action::SetError(Some(err)));
                    }
                }
            }
            Err(e) => {
                self.store.dispatch(Action::SetError(Some(e.to_string())));
            }
        }

        Ok(())
    }

    fn handle_traceroute(&self, cmd: &AppCommand, device: &Device) -> Result<()> {
        let exec = self.commander.traceroute(device);
        match exec {
            Ok(output) => {
                self.store
                    .dispatch(Action::UpdateCommandOutput((cmd.clone(), output)));
                self.store.dispatch(Action::SetCommandInProgress(None));
            }
            Err(err) => {
                self.store.dispatch(Action::SetError(Some(err.to_string())));
            }
        }

        Ok(())
    }

    fn handle_browse(&self, args: &BrowseArgs) -> Result<()> {
        self.pause_ui()?;

        let res = self.commander.browse(args);

        self.tx.send(Event::ResumeUI)?;

        match res {
            Ok((status, err)) => {
                if !status.success() {
                    if let Some(stderr) = err {
                        let mut stderr_output = String::new();
                        let mut stderr_reader = BufReader::new(stderr);
                        stderr_reader.read_to_string(&mut stderr_output)?;
                        self.store.dispatch(Action::SetError(Some(stderr_output)));
                    } else {
                        let err = String::from("lynx command failed");
                        self.store.dispatch(Action::SetError(Some(err)));
                    }
                }
            }
            Err(e) => {
                self.store.dispatch(Action::SetError(Some(e.to_string())));
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
            .dispatch(Action::SetCommandInProgress(Some(cmd.clone())));

        match &cmd {
            AppCommand::Ssh(device, device_config) => self.handle_ssh(device, device_config)?,
            AppCommand::TraceRoute(device) => self.handle_traceroute(&cmd, device)?,
            AppCommand::Browse(args) => self.handle_browse(args)?,
        }

        self.store.dispatch(Action::SetCommandInProgress(None));

        Ok(())
    }
}

#[cfg(test)]
#[path = "./manager_tests.rs"]
mod tests;
