//! Processes UI events and executes external commands.

use color_eyre::eyre::{Result, eyre};
use r_lanlib::scanners::Device;
use std::{
    io::{BufReader, Read},
    process,
    sync::{Arc, RwLock},
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

#[derive(Default)]
pub struct CtrlCHandler {
    block: Arc<RwLock<bool>>,
}

impl CtrlCHandler {
    pub fn intercept(&self) -> Result<()> {
        // captures ctrl-c only in main thread so when we drop down to shell
        // commands like ssh, we will pause the key handler for ctrl-c in app
        // and capture ctrl-c here to prevent exiting app and just let ctrl-c
        // be handled by the command being executed, which should return us
        // to our app where we can restart our ui and key-handlers
        let block = Arc::clone(&self.block);

        ctrlc::set_handler(move || {
            if let Ok(blocked) = block.read() {
                if *blocked {
                    println!("captured ctrl-c!");
                } else {
                    process::exit(1);
                }
            } else {
                process::exit(1);
            }
        })
        .map_err(|err| eyre!("failed to set ctrl-c handler: {}", err))
    }

    pub fn block(&self) -> Result<()> {
        let mut blocked = self.block.write().map_err(|err| {
            eyre!("failed to get write lock on ctrl-c block setting: {}", err)
        })?;
        *blocked = true;
        Ok(())
    }

    pub fn unblock(&self) -> Result<()> {
        let mut blocked = self.block.write().map_err(|err| {
            eyre!("failed to get write lock on ctrl-c block setting: {}", err)
        })?;
        *blocked = false;
        Ok(())
    }
}

/// Runs the event loop, handling UI pause/resume and command execution.
pub struct MainEventHandler {
    store: Arc<Store>,
    executor: Box<dyn ShellExecutor>,
    ipc: MainIpc,
    ctrlc_handler: CtrlCHandler,
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
            ctrlc_handler: CtrlCHandler::default(),
        }
    }

    /// Runs the main event loop, blocking until a quit message is received.
    /// Dispatches commands to the appropriate handler (SSH, traceroute, browse).
    pub fn process_events(&self) -> Result<()> {
        self.ctrlc_handler.intercept()?;
        loop {
            if let Ok(evt) = self.ipc.rx.recv() {
                // event loop
                match evt {
                    MainMessage::ExecCommand(cmd) => {
                        let _ = self.handle_cmd(cmd);
                    }
                    MainMessage::Quit(error) => {
                        if let Some(message) = error {
                            return Err(eyre!(message));
                        } else {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn pause_ui(&self) -> Result<()> {
        self.ipc.tx.send(RendererMessage::PauseUI)?;
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
        self.ipc.tx.send(RendererMessage::ResumeUI)?;
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
        self.resume_ui()?;

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
