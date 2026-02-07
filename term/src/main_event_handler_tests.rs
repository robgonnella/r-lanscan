use color_eyre::eyre::eyre;
use nanoid::nanoid;
use pnet::util::MacAddr;
use r_lanlib::scanners::{Device, PortSet};
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
    net::Ipv4Addr,
    os::{fd::OwnedFd, unix::process::ExitStatusExt},
    process::{ChildStderr, ExitStatus, Output},
    sync::Mutex,
};

use crate::{
    config::{Config, ConfigManager, DeviceConfig},
    ipc::traits::{MockIpcReceiver, MockIpcSender},
    shell::traits::MockShellExecutor,
    ui::store::StateGetter,
};

use super::*;

struct SetUpReturn {
    store: Arc<Store>,
    main_handler: MainEventHandler,
}

fn setup(
    conf_manager: ConfigManager,
    mock_executor: MockShellExecutor,
    mock_sender: MockIpcSender<RendererMessage>,
    mock_receiver: MockIpcReceiver<MainMessage>,
) -> SetUpReturn {
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();
    let config = Config::new(user, identity, cidr);
    let store =
        Arc::new(Store::new(Arc::new(Mutex::new(conf_manager)), config));
    let main_ipc = MainIpc::new(Box::new(mock_sender), Box::new(mock_receiver));
    let main_handler = MainEventHandler::new(
        Arc::clone(&store),
        Box::new(mock_executor),
        main_ipc,
    );
    SetUpReturn {
        store,
        main_handler,
    }
}

fn tear_down(path: &str) {
    fs::remove_file(path).unwrap();
}

#[test]
fn handles_ssh_command_err() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_ssh()
        .returning(|_, _| Err(eyre!("mock error")));

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Ssh(device, device_config))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());

    tear_down(tmp_path.as_str());
}

#[test]
fn handles_ssh_command_ok() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_ssh()
        .returning(|_, _| Ok((ExitStatus::default(), None)));

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Ssh(device, device_config))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_none());
    tear_down(tmp_path.as_str());
}

#[test]
fn handles_ssh_command_ok_err() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor.expect_ssh().returning(|_, _| {
        let mut tmpfile: File = tempfile::tempfile().unwrap();
        writeln!(tmpfile, "test error").unwrap();
        tmpfile.seek(SeekFrom::Start(0)).unwrap();

        let status = ExitStatusExt::from_raw(1);
        let fd = OwnedFd::from(tmpfile);
        let mock_stderr = ChildStderr::from(fd);

        Ok((status, Some(mock_stderr)))
    });

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Ssh(device, device_config))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());

    let err = state.error.unwrap();
    assert_eq!(err, "test error\n");

    tear_down(tmp_path.as_str());
}

#[test]
fn handles_ssh_command_ok_err_empty() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor.expect_ssh().returning(|_, _| {
        let status = ExitStatusExt::from_raw(1);
        Ok((status, None))
    });

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Ssh(device, device_config))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());

    tear_down(tmp_path.as_str());
}

#[test]
fn handles_traceroute_command_err() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mock_sender = MockIpcSender::<RendererMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_traceroute()
        .returning(|_| Err(eyre!("mock error")));

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    test.main_handler
        .handle_cmd(AppCommand::TraceRoute(device))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());
    tear_down(tmp_path.as_str());
}

#[test]
fn handles_traceroute_command_ok() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mock_sender = MockIpcSender::<RendererMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor.expect_traceroute().returning(|_| {
        let o = Output {
            status: ExitStatusExt::from_raw(0),
            stdout: "this is some output".as_bytes().to_vec(),
            stderr: vec![],
        };
        Ok(o)
    });

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let expected_output = Output {
        status: ExitStatusExt::from_raw(0),
        stdout: "this is some output".as_bytes().to_vec(),
        stderr: vec![],
    };

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    test.main_handler
        .handle_cmd(AppCommand::TraceRoute(device.clone()))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_none());
    assert!(state.cmd_output.is_some());

    let output = state.cmd_output.unwrap();
    assert_eq!(output.0, AppCommand::TraceRoute(device));
    assert_eq!(output.1, expected_output);
    tear_down(tmp_path.as_str());
}

#[test]
fn handles_browse_command_err() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_browse()
        .returning(|_| Err(eyre!("mock error")));

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Browse(BrowseArgs {
            device,
            port: 80,
            use_lynx: false,
        }))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());
    tear_down(tmp_path.as_str());
}

#[test]
fn handles_browse_command_ok() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_browse()
        .returning(|_| Ok((ExitStatus::default(), None)));

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Browse(BrowseArgs {
            device,
            port: 80,
            use_lynx: false,
        }))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_none());
    tear_down(tmp_path.as_str());
}

#[test]
fn handles_browse_command_ok_err() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor.expect_browse().returning(|_| {
        let mut tmpfile: File = tempfile::tempfile().unwrap();
        writeln!(tmpfile, "test error").unwrap();
        tmpfile.seek(SeekFrom::Start(0)).unwrap();

        let status = ExitStatusExt::from_raw(1);
        let fd = OwnedFd::from(tmpfile);
        let mock_stderr = ChildStderr::from(fd);

        Ok((status, Some(mock_stderr)))
    });

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Browse(BrowseArgs {
            device,
            port: 80,
            use_lynx: false,
        }))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());

    let err = state.error.unwrap();
    assert_eq!(err, "test error\n");
    tear_down(tmp_path.as_str());
}

#[test]
fn handles_browse_command_ok_err_empty() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor.expect_browse().returning(|_| {
        let status = ExitStatusExt::from_raw(1);
        Ok((status, None))
    });

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    test.main_handler
        .handle_cmd(AppCommand::Browse(BrowseArgs {
            device,
            port: 80,
            use_lynx: false,
        }))
        .unwrap();

    let state = test.store.get_state().unwrap();
    assert!(state.error.is_some());
    tear_down(tmp_path.as_str());
}

#[test]
fn listens_for_events() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_traceroute()
        .returning(|_| Err(eyre!("mock error")));

    mock_receiver
        .expect_recv()
        .returning(|| {
            Ok(MainMessage::ExecCommand(AppCommand::TraceRoute(Device {
                hostname: "Hostname".to_string(),
                ip: Ipv4Addr::new(10, 10, 10, 1),
                mac: MacAddr::default(),
                vendor: "Vendor".to_string(),
                is_current_host: false,
                open_ports: PortSet::new(),
            })))
        })
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::Quit))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    test.main_handler.process_events().unwrap();

    let state = test.store.get_state().unwrap();

    assert!(state.error.is_some());

    tear_down(tmp_path.as_str());
}

#[test]
fn pause_ui_handles_quit() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mock_executor = MockShellExecutor::new();

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::Quit))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    let result = test
        .main_handler
        .handle_cmd(AppCommand::Ssh(device, device_config));

    assert!(result.is_err());

    tear_down(tmp_path.as_str());
}

#[test]
fn resume_ui_handles_quit() {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();

    let conf_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let mut mock_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();
    let mut mock_executor = MockShellExecutor::new();

    mock_executor
        .expect_ssh()
        .returning(|_, _| Ok((ExitStatus::default(), None)));

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()));

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::Quit))
        .times(1);

    let test = setup(conf_manager, mock_executor, mock_sender, mock_receiver);

    let device = Device {
        hostname: "Hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        vendor: "Vendor".to_string(),
        is_current_host: false,
        open_ports: PortSet::new(),
    };

    let device_config = DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    };

    let result = test
        .main_handler
        .handle_cmd(AppCommand::Ssh(device, device_config));

    assert!(result.is_err());

    tear_down(tmp_path.as_str());
}
