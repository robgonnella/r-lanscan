use std::{
    io::{BufReader, Read},
    process::{Command as ShellCommand, Stdio},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex, MutexGuard,
    },
};

use color_eyre::eyre::{Context, Result};

use crate::ui::store::{action::Action, store::Store};

use super::types::{Command as AppCommand, Event};

pub struct EventManager {
    tx: Sender<Event>,
    rx: Arc<Mutex<Receiver<Event>>>,
    store: Arc<Store>,
}

impl EventManager {
    pub fn new(tx: Sender<Event>, rx: Receiver<Event>, store: Arc<Store>) -> Self {
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            store,
        }
    }

    pub fn start_event_loop(&self) -> Result<()> {
        ctrlc::set_handler(move || println!("captured ctrl-c in event thread!"))
            .expect("Error setting Ctrl-C handler");

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
        let state = self.store.get_state();

        if state.cmd_in_progress.is_some() {
            return Ok(());
        }

        self.store
            .dispatch(Action::SetCommandInProgress(Some(cmd.clone())));

        match cmd.clone() {
            AppCommand::SSH(device, device_config) => {
                self.tx.send(Event::PauseUI)?;
                loop {
                    if let Ok(evt) = rx.recv() {
                        if evt == Event::UIPaused {
                            break;
                        }
                    }
                }
                let mut handle = ShellCommand::new("ssh")
                    .arg("-i")
                    .arg(device_config.ssh_identity_file)
                    .arg(format!("{}@{}", device_config.ssh_user, device.ip))
                    .arg("-p")
                    .arg(device_config.ssh_port.to_string())
                    .stderr(Stdio::piped())
                    .spawn()
                    .wrap_err("failed to start ssh command")?;

                let res = handle.wait();

                self.tx.send(Event::ResumeUI)?;

                match res {
                    Ok(status) => {
                        if !status.success() {
                            if handle.stderr.is_some() {
                                let mut stderr_output = String::new();
                                let stderr = handle.stderr.unwrap();
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
            AppCommand::TRACEROUTE(device) => {
                let exec = ShellCommand::new("traceroute").arg(device.ip).output();
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
            AppCommand::BROWSE(device, port) => {
                self.tx.send(Event::PauseUI)?;
                loop {
                    if let Ok(evt) = rx.recv() {
                        if evt == Event::UIPaused {
                            break;
                        }
                    }
                }
                let mut handle = ShellCommand::new("lynx")
                    .arg(format!("{}:{}", device.ip, port))
                    .stderr(Stdio::piped())
                    .env("TERM", "xterm")
                    .spawn()
                    .wrap_err("failed to start lynx browser")?;

                let res = handle.wait();

                self.tx.send(Event::ResumeUI)?;

                match res {
                    Ok(status) => {
                        if !status.success() {
                            if handle.stderr.is_some() {
                                let mut stderr_output = String::new();
                                let stderr = handle.stderr.unwrap();
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
