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
    use nanoid::nanoid;
    use r_lanlib::scanners::Device;
    use std::{
        fs::{self, File},
        io::{Seek, SeekFrom, Write},
        os::{fd::OwnedFd, unix::process::ExitStatusExt},
        process::{ChildStderr, ExitStatus, Output},
    };

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
        conf_manager: ConfigManager,
        commander: Commander,
    ) -> (
        Sender<Event>,
        Arc<Mutex<Receiver<Event>>>,
        Arc<Store>,
        EventManager,
    ) {
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

    fn tear_down(path: &str) {
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn handles_ssh_command_err() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander
            .expect_ssh()
            .returning(|_, _| Err(Box::from("mock error")));

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_ssh_command_ok() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander
            .expect_ssh()
            .returning(|_, _| Ok((ExitStatus::default(), None)));

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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
        assert!(state.error.is_none());

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_ssh_command_ok_err() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander.expect_ssh().returning(|_, _| {
            let mut tmpfile: File = tempfile::tempfile().unwrap();
            writeln!(tmpfile, "test error").unwrap();
            tmpfile.seek(SeekFrom::Start(0)).unwrap();

            let status = ExitStatusExt::from_raw(1);
            let fd = OwnedFd::from(tmpfile);
            let mock_stderr = ChildStderr::from(fd);

            Ok((status, Some(mock_stderr)))
        });

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        let err = state.error.unwrap();
        assert_eq!(err, "test error\n");

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_ssh_command_ok_err_empty() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander.expect_ssh().returning(|_, _| {
            let status = ExitStatusExt::from_raw(1);
            Ok((status, None))
        });

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_traceroute_command_err() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander
            .expect_traceroute()
            .returning(|_| Err(Box::from("mock error")));

        let (_sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_traceroute_command_ok() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        let expected_output = Output {
            status: ExitStatusExt::from_raw(0),
            stdout: "this is some output".as_bytes().to_vec(),
            stderr: vec![],
        };

        mock_commander.expect_traceroute().returning(|_| {
            let o = Output {
                status: ExitStatusExt::from_raw(0),
                stdout: "this is some output".as_bytes().to_vec(),
                stderr: vec![],
            };
            Ok(o)
        });

        let (_sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

        let device = Device {
            hostname: "Hostname".to_string(),
            ip: "IP".to_string(),
            mac: "MAC".to_string(),
            vendor: "Vendor".to_string(),
            is_current_host: false,
        };

        let rx = receiver.lock().unwrap();
        let res = evt_manager.handle_cmd(rx, AppCommand::TRACEROUTE(device.clone()));
        assert!(res.is_ok());

        let state = store.get_state();
        assert!(state.error.is_none());

        assert!(state.cmd_output.is_some());

        let output = state.cmd_output.unwrap();
        assert_eq!(output.0, AppCommand::TRACEROUTE(device));
        assert_eq!(output.1, expected_output);

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_browse_command_err() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander
            .expect_lynx()
            .returning(|_, _| Err(Box::from("mock error")));

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_browse_command_ok() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander
            .expect_lynx()
            .returning(|_, _| Ok((ExitStatus::default(), None)));

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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
        assert!(state.error.is_none());

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_browse_command_ok_err() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander.expect_lynx().returning(|_, _| {
            let mut tmpfile: File = tempfile::tempfile().unwrap();
            writeln!(tmpfile, "test error").unwrap();
            tmpfile.seek(SeekFrom::Start(0)).unwrap();

            let status = ExitStatusExt::from_raw(1);
            let fd = OwnedFd::from(tmpfile);
            let mock_stderr = ChildStderr::from(fd);

            Ok((status, Some(mock_stderr)))
        });

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        let err = state.error.unwrap();
        assert_eq!(err, "test error\n");

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn handles_browse_command_ok_err_empty() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander.expect_lynx().returning(|_, _| {
            let status = ExitStatusExt::from_raw(1);
            Ok((status, None))
        });

        let (sender, receiver, store, evt_manager) = setup(conf_manager, mock_commander);

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

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn listens_for_events() {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = ConfigManager::new(tmp_path.as_str());

        let mut mock_commander = Commander::default();

        mock_commander
            .expect_traceroute()
            .returning(|_| Err(Box::from("mock error")));

        let (sender, _receiver, store, evt_manager) = setup(conf_manager, mock_commander);

        let device = Device {
            hostname: "Hostname".to_string(),
            ip: "IP".to_string(),
            mac: "MAC".to_string(),
            vendor: "Vendor".to_string(),
            is_current_host: false,
        };

        sender
            .send(Event::ExecCommand(AppCommand::TRACEROUTE(device)))
            .unwrap();

        sender.send(Event::Quit).unwrap();

        let res = evt_manager.start_event_loop();

        assert!(res.is_ok());

        let state = store.get_state();

        assert!(state.error.is_some());

        tear_down(tmp_path.as_str());
    }
}
