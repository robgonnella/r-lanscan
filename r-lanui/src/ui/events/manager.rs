use log::*;
use std::{
    process::Command as ShellCommand,
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
                info!("starting ssh to {}", device.ip);
                let mut handle = ShellCommand::new("ssh")
                    .arg("-i")
                    .arg(device_config.ssh_identity_file)
                    .arg(format!("{}@{}", device_config.ssh_user, device.ip))
                    .arg("-p")
                    .arg(device_config.ssh_port.to_string())
                    .spawn()
                    .wrap_err("failed to start ssh command")?;
                handle.wait().wrap_err("command failed")?;
                debug!("restarting terminal");
                self.tx.send(Event::ResumeUI)?;
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
                let _ = self.tx.send(Event::PauseUI);
                loop {
                    if let Ok(evt) = rx.recv() {
                        if evt == Event::UIPaused {
                            break;
                        }
                    }
                }
                info!("starting browser for {}:{}", device.ip, port);
                let mut handle = ShellCommand::new("lynx")
                    .arg(format!("{}:{}", device.ip, port))
                    .spawn()
                    .wrap_err("failed to start lynx browser")?;
                handle.wait().wrap_err("shell command failed")?;
                let _ = self.tx.send(Event::ResumeUI);
            }
        }

        self.store.dispatch(Action::SetCommandInProgress(None));

        Ok(())
    }
}
