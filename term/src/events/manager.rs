use color_eyre::eyre::Result;
use mockall_double::double;
use std::{
    io::{BufReader, Read},
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{Receiver, Sender},
    },
};

use crate::ui::store::{Dispatcher, Store, action::Action};

use super::types::{Command as AppCommand, Event};

// double allows tests to use the mocked version of Commander
#[double]
use super::commander::Commander;

pub struct EventManager {
    tx: Sender<Event>,
    rx: Arc<Mutex<Receiver<Event>>>,
    store: Arc<Store>,
    commander: Commander,
}

impl EventManager {
    pub fn new(tx: Sender<Event>, rx: Receiver<Event>, store: Arc<Store>) -> Self {
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            store,
            commander: Commander::new(),
        }
    }

    pub fn start_event_loop(&self) -> Result<()> {
        let rx = Arc::clone(&self.rx);

        loop {
            let locked_rx = rx.lock().unwrap();
            if let Ok(evt) = locked_rx.recv() {
                // event loop
                match evt {
                    Event::ExecCommand(cmd) => {
                        let _ = self.handle_cmd(locked_rx, cmd);
                    }
                    Event::Quit => break,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn handle_cmd(&self, rx: MutexGuard<'_, Receiver<Event>>, cmd: AppCommand) -> Result<()> {
        let state = self.store.get_state()?;

        if state.cmd_in_progress.is_some() {
            return Ok(());
        }

        self.store
            .dispatch(Action::SetCommandInProgress(Some(cmd.clone())));

        match cmd.clone() {
            AppCommand::Ssh(device, device_config) => {
                self.tx.send(Event::PauseUI)?;
                loop {
                    if let Ok(evt) = rx.recv()
                        && evt == Event::UIPaused
                    {
                        break;
                    }
                }

                let res = self.commander.ssh(device, device_config);

                self.tx.send(Event::ResumeUI)?;

                match res {
                    Ok((status, err)) => {
                        if !status.success() {
                            if let Some(stderr) = err {
                                let mut stderr_output = String::new();
                                let mut stderr_reader = BufReader::new(stderr);
                                stderr_reader.read_to_string(&mut stderr_output).unwrap();
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
            }
            AppCommand::TraceRoute(device) => {
                let exec = self.commander.traceroute(device);
                match exec {
                    Ok(output) => {
                        self.store
                            .dispatch(Action::UpdateCommandOutput((cmd, output)));
                        self.store.dispatch(Action::SetCommandInProgress(None));
                    }
                    Err(err) => {
                        self.store.dispatch(Action::SetError(Some(err.to_string())));
                    }
                }
            }
            AppCommand::Browse(args) => {
                self.tx.send(Event::PauseUI)?;
                loop {
                    if let Ok(evt) = rx.recv()
                        && evt == Event::UIPaused
                    {
                        break;
                    }
                }

                let res = self.commander.browse(args);

                self.tx.send(Event::ResumeUI)?;

                match res {
                    Ok((status, err)) => {
                        if !status.success() {
                            if let Some(stderr) = err {
                                let mut stderr_output = String::new();
                                let mut stderr_reader = BufReader::new(stderr);
                                stderr_reader.read_to_string(&mut stderr_output).unwrap();
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
            }
        }

        self.store.dispatch(Action::SetCommandInProgress(None));

        Ok(())
    }
}

#[cfg(test)]
#[path = "./manager_tests.rs"]
mod tests;
