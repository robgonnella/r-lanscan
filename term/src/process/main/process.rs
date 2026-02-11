//! Processes UI events and executes external commands.

use color_eyre::eyre::{Result, eyre};
use derive_builder::Builder;
use r_lanlib::scanners::Device;
use std::{
    cell::RefCell,
    io::{BufReader, Read},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    config::{ConfigManager, DeviceConfig},
    ipc::{
        main::MainIpc,
        message::{
            Command as AppCommand, MainMessage, NetworkMessage, RendererMessage,
        },
    },
    process::main::ctrl_c::CtrlCHandler,
    shell::traits::{BrowseArgs, ShellExecutor},
    store::{Dispatcher, StateGetter, Store, action::Action},
};

/// Runs the event loop, handling UI pause/resume and command execution.
#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct MainProcess {
    config_manager: RefCell<ConfigManager>,
    store: Rc<Store>,
    executor: Box<dyn ShellExecutor>,
    ipc: MainIpc,
    #[builder(default)]
    ctrlc_handler: CtrlCHandler,
}

impl MainProcess {
    /// Creates a new builder for main process
    pub fn builder() -> MainProcessBuilder {
        MainProcessBuilder::default()
    }

    /// Runs the main event loop, blocking until a quit message is received.
    /// Dispatches commands to the appropriate handler (SSH, traceroute, browse).
    pub fn process_events(&self) -> Result<()> {
        self.ctrlc_handler.intercept()?;
        loop {
            if let Ok(evt) = self.ipc.rx.recv() {
                log::info!("received event --> {:#?}", evt);
                // event loop
                match evt {
                    MainMessage::ExecCommand(cmd) => {
                        self.handle_cmd(cmd)?;
                    }
                    MainMessage::Quit(error) => {
                        self.ipc.network_tx.send(NetworkMessage::Quit)?;
                        if let Some(message) = error {
                            return Err(eyre!(message));
                        } else {
                            return Ok(());
                        }
                    }
                    MainMessage::ArpStart => {
                        self.store.dispatch(Action::UpdateMessage(Some(
                            "ARP scanning in progress...".into(),
                        )));
                    }
                    MainMessage::ArpUpdate(device) => {
                        self.store.dispatch(Action::AddDevice(device));
                    }
                    MainMessage::ArpDone => {
                        self.store.dispatch(Action::Log(
                            "ARP scanning complete".into(),
                        ));
                    }
                    MainMessage::SynStart => {
                        self.store.dispatch(Action::UpdateMessage(Some(
                            "SYN scanning in progress...".into(),
                        )));
                    }
                    MainMessage::SynUpdate(device) => {
                        self.store.dispatch(Action::AddDevice(device));
                    }
                    MainMessage::SynDone => {
                        self.store.dispatch(Action::UpdateMessage(None));
                    }
                    MainMessage::FullScanResult(devices) => {
                        self.store.dispatch(Action::UpdateAllDevices(devices));
                    }
                    MainMessage::ActionSync(action) => {
                        self.store.dispatch(action.as_ref().to_owned());
                        self.handle_post_action_sync(action)?;
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_post_action_sync(&self, action: Box<Action>) -> Result<()> {
        let new_config = if let Action::Sync(act) = action.as_ref() {
            match act.as_ref() {
                Action::UpdateConfig(_) => {
                    Some(self.store.get_state().config.clone())
                }
                Action::UpdateDeviceConfig(_) => {
                    Some(self.store.get_state().config.clone())
                }
                _ => None,
            }
        } else {
            None
        };

        if let Some(config) = new_config {
            self.config_manager
                .borrow_mut()
                .update_config(config.clone())?;

            self.ipc
                .network_tx
                .send(NetworkMessage::ConfigUpdate(config))?;
        }

        Ok(())
    }

    fn pause_ui(&self) -> Result<()> {
        self.ipc.renderer_tx.send(RendererMessage::PauseUI)?;
        let start = Instant::now();
        let timeout = Duration::from_millis(5000);
        loop {
            if start.elapsed() >= timeout {
                return Ok(());
            }
            if let Ok(evt) = self.ipc.rx.recv() {
                match evt {
                    MainMessage::UIPaused => {
                        self.ctrlc_handler.block()?;
                        return Ok(());
                    }
                    MainMessage::Quit(error) => {
                        if let Some(message) = error {
                            return Err(eyre!(
                                "quit received during pause: details: {}",
                                message
                            ));
                        } else {
                            return Err(eyre!("quit received during pause"));
                        }
                    }
                    other => {
                        log::debug!("pause_ui: ignored {:?}", other);
                    }
                }
            }
        }
    }

    fn resume_ui(&self) -> Result<()> {
        self.ipc.renderer_tx.send(RendererMessage::ResumeUI)?;
        let start = Instant::now();
        let timeout = Duration::from_millis(5000);
        loop {
            if start.elapsed() >= timeout {
                return Ok(());
            }
            if let Ok(evt) = self.ipc.rx.recv() {
                match evt {
                    MainMessage::UIResumed => {
                        self.ctrlc_handler.unblock()?;
                        return Ok(());
                    }
                    MainMessage::Quit(error) => {
                        if let Some(message) = error {
                            return Err(eyre!(
                                "quit received during resume: details: {}",
                                message
                            ));
                        } else {
                            return Err(eyre!("quit received during resume"));
                        }
                    }
                    other => {
                        log::debug!("resume_ui: ignored {:?}", other);
                    }
                }
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
        self.resume_ui()?;
        match res {
            Ok((status, err)) => {
                if !status.success() {
                    if let Some(stderr) = err {
                        let mut stderr_output = String::new();
                        let mut stderr_reader = BufReader::new(stderr);
                        stderr_reader.read_to_string(&mut stderr_output)?;
                        self.store
                            .dispatch(Action::SetError(Some(stderr_output)));
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
                )));
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
        let res = self.executor.browse(args);
        self.resume_ui()?;

        match res {
            Ok((status, err)) => {
                if !status.success() {
                    if let Some(stderr) = err {
                        let mut stderr_output = String::new();
                        let mut stderr_reader = BufReader::new(stderr);
                        stderr_reader.read_to_string(&mut stderr_output)?;
                        self.store
                            .dispatch(Action::SetError(Some(stderr_output)));
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
        let state = self.store.get_state();

        if state.cmd_in_progress.is_some() {
            return Ok(());
        }

        self.store
            .dispatch(Action::SetCommandInProgress(Some(cmd.clone())));

        match &cmd {
            AppCommand::Ssh(device, device_config) => {
                self.handle_ssh(device, device_config)?
            }
            AppCommand::TraceRoute(device) => {
                self.handle_traceroute(&cmd, device)?
            }
            AppCommand::Browse(args) => self.handle_browse(args)?,
        }

        self.store.dispatch(Action::SetCommandInProgress(None));

        Ok(())
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod process_tests;
