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
