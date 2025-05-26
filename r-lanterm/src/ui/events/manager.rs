use color_eyre::eyre::Result;
use mockall_double::double;
use std::{
    io::{BufReader, Read},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex, MutexGuard,
    },
};

use crate::ui::store::{action::Action, store::Store};

use super::types::{Command as AppCommand, Event};

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

                let res = self.commander.ssh(device, device_config);

                self.tx.send(Event::ResumeUI)?;

                match res {
                    Ok((status, err)) => {
                        if !status.success() {
                            if err.is_some() {
                                let mut stderr_output = String::new();
                                let stderr = err.unwrap();
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
            AppCommand::BROWSE(device, port) => {
                self.tx.send(Event::PauseUI)?;
                loop {
                    if let Ok(evt) = rx.recv() {
                        if evt == Event::UIPaused {
                            break;
                        }
                    }
                }

                let res = self.commander.lynx(device, port);

                self.tx.send(Event::ResumeUI)?;

                match res {
                    Ok((status, err)) => {
                        if !status.success() {
                            if err.is_some() {
                                let mut stderr_output = String::new();
                                let stderr = err.unwrap();
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
mod tests {
    use r_lanlib::scanners::Device;

    use crate::config::{ConfigManager, DeviceConfig};

    use super::*;

    fn new_with_commander(
        tx: Sender<Event>,
        rx: Arc<Mutex<Receiver<Event>>>,
        store: Arc<Store>,
        commander: Commander,
    ) -> EventManager {
        EventManager {
            tx,
            rx,
            store,
            commander,
        }
    }

    fn setup(
        commander: Commander,
    ) -> (
        Sender<Event>,
        Arc<Mutex<Receiver<Event>>>,
        Arc<Store>,
        EventManager,
    ) {
        let conf_manager = ConfigManager::new("./generated");
        let store = Arc::new(Store::new(Arc::new(Mutex::new(conf_manager))));
        let (tx, rx) = std::sync::mpsc::channel::<Event>();
        let arc_rx = Arc::new(Mutex::new(rx));
        let evt_manager = new_with_commander(
            tx.clone(),
            Arc::clone(&arc_rx),
            Arc::clone(&store),
            commander,
        );
        return (tx, arc_rx, store, evt_manager);
    }

    #[test]
    fn handles_ssh_command() {
        let mut mock_commander = Commander::default();

        mock_commander
            .expect_ssh()
            .returning(|_, _| Err(Box::from("mock error")));

        let (sender, receiver, store, evt_manager) = setup(mock_commander);

        let device = Device {
            hostname: "Hostname".to_string(),
            ip: "IP".to_string(),
            mac: "MAC".to_string(),
            vendor: "Vendor".to_string(),
            is_current_host: false,
        };

        let device_config = DeviceConfig {
            id: "device_id".to_string(),
            ssh_port: 22,
            ssh_identity_file: "id_rsa".to_string(),
            ssh_user: "user".to_string(),
        };

        let res = sender.send(Event::UIPaused);
        assert!(res.is_ok());

        let rx = receiver.lock().unwrap();
        let res = evt_manager.handle_cmd(rx, AppCommand::SSH(device, device_config));
        assert!(res.is_ok());

        let state = store.get_state();
        assert!(state.error.is_some());
    }

    #[test]
    fn handles_traceroute_command() {
        let mut mock_commander = Commander::default();

        mock_commander
            .expect_traceroute()
            .returning(|_| Err(Box::from("mock error")));

        let (_sender, receiver, store, evt_manager) = setup(mock_commander);

        let device = Device {
            hostname: "Hostname".to_string(),
            ip: "IP".to_string(),
            mac: "MAC".to_string(),
            vendor: "Vendor".to_string(),
            is_current_host: false,
        };

        let rx = receiver.lock().unwrap();
        let res = evt_manager.handle_cmd(rx, AppCommand::TRACEROUTE(device));
        assert!(res.is_ok());

        let state = store.get_state();
        assert!(state.error.is_some());
    }

    #[test]
    fn handles_browse_command() {
        let mut mock_commander = Commander::default();

        mock_commander
            .expect_lynx()
            .returning(|_, _| Err(Box::from("mock error")));

        let (sender, receiver, store, evt_manager) = setup(mock_commander);

        let device = Device {
            hostname: "Hostname".to_string(),
            ip: "IP".to_string(),
            mac: "MAC".to_string(),
            vendor: "Vendor".to_string(),
            is_current_host: false,
        };

        let res = sender.send(Event::UIPaused);
        assert!(res.is_ok());

        let rx = receiver.lock().unwrap();
        let res = evt_manager.handle_cmd(rx, AppCommand::BROWSE(device, 80));
        assert!(res.is_ok());

        let state = store.get_state();
        assert!(state.error.is_some());
    }
}
