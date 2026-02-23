use color_eyre::eyre::eyre;
use nanoid::nanoid;
use r_lanlib::scanners::Device;
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
    net::Ipv4Addr,
    os::{fd::OwnedFd, unix::process::ExitStatusExt},
    process::{ChildStderr, ExitStatus, Output},
    sync::{Arc, mpsc::channel},
};

use crate::{
    config::DeviceConfig,
    ipc::traits::{MockIpcReceiver, MockIpcSender},
    shell::traits::MockShellExecutor,
    store::{reducer::StoreReducer, state::State},
};

use super::*;

struct SetUpReturn {
    tmp_path: String,
    store: Rc<Store>,
    main_process: MainProcess,
    main_rx: std::sync::mpsc::Receiver<MainMessage>,
}

fn setup(
    mock_executor: MockShellExecutor,
    mock_render_sender: MockIpcSender<RendererMessage>,
    mock_network_sender: MockIpcSender<NetworkMessage>,
    mock_receiver: MockIpcReceiver<MainMessage>,
) -> SetUpReturn {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());

    let config_manager = ConfigManager::builder()
        .default_cidr("192.168.1.1/24")
        .default_identity("/home/user/.ssh/id_rsa")
        .default_user("user")
        .path(tmp_path.clone())
        .build()
        .unwrap();

    let store = Rc::new(Store::new(State::default(), StoreReducer::boxed()));

    let (main_tx, main_rx) = channel::<MainMessage>();

    let main_ipc = MainIpc::new(
        Box::new(mock_render_sender),
        Box::new(mock_network_sender),
        Box::new(mock_receiver),
        main_tx,
    );

    let main_process = MainProcess::builder()
        .executor(Arc::new(mock_executor))
        .config_manager(RefCell::new(config_manager))
        .ipc(main_ipc)
        .store(store.clone())
        .build()
        .unwrap();

    SetUpReturn {
        tmp_path,
        store,
        main_process,
        main_rx,
    }
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

fn make_device() -> Device {
    Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        vendor: "vendor".to_string(),
        ..Device::default()
    }
}

fn make_device_config() -> DeviceConfig {
    DeviceConfig {
        id: "device_id".to_string(),
        ssh_port: 22,
        ssh_identity_file: "id_rsa".to_string(),
        ssh_user: "user".to_string(),
    }
}

// -- handle_cmd tests --

#[test]
fn handle_cmd_ignores_when_cmd_in_progress() {
    let mock_executor = MockShellExecutor::new();
    let mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    test.store.dispatch(Action::SetCommandInProgress(Some(
        AppCommand::TraceRoute(device.clone()),
    )));

    let result = test.main_process.handle_cmd(AppCommand::TraceRoute(device));
    assert!(result.is_ok());

    // executor was never called (no expectations set on it
    // so it would panic if called)

    tear_down(test.tmp_path);
}

// -- handle_traceroute tests --

#[test]
fn handle_traceroute_dispatches_output_on_success() {
    let mut mock_executor = MockShellExecutor::new();
    let mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor.expect_traceroute().returning(|_| {
        Ok(Output {
            status: ExitStatusExt::from_raw(0),
            stdout: "trace output".as_bytes().to_vec(),
            stderr: vec![],
        })
    });

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let cmd = AppCommand::TraceRoute(device.clone());

    test.main_process.handle_traceroute(&cmd, &device).unwrap();

    // Result arrives asynchronously via the main channel
    match test.main_rx.recv().unwrap() {
        MainMessage::CommandDone(done_cmd, Ok(output)) => {
            test.store
                .dispatch(Action::UpdateCommandOutput((done_cmd, output)));
            test.store.dispatch(Action::SetCommandInProgress(None));
        }
        other => panic!("unexpected message: {other:?}"),
    }

    let state = test.store.get_state();
    assert!(state.cmd_output.is_some());

    let (out_cmd, output) = state.cmd_output.as_ref().unwrap();
    assert_eq!(*out_cmd, cmd);
    assert_eq!(output.stdout, "trace output".as_bytes());

    tear_down(test.tmp_path);
}

#[test]
fn handle_traceroute_sets_error_on_failure() {
    let mut mock_executor = MockShellExecutor::new();
    let mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor
        .expect_traceroute()
        .returning(|_| Err(eyre!("traceroute failed")));

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let cmd = AppCommand::TraceRoute(device.clone());

    test.main_process.handle_traceroute(&cmd, &device).unwrap();

    // Result arrives asynchronously via the main channel
    match test.main_rx.recv().unwrap() {
        MainMessage::CommandDone(_, Err(err)) => {
            test.store.dispatch(Action::SetError(Some(err)));
            test.store.dispatch(Action::SetCommandInProgress(None));
        }
        other => panic!("unexpected message: {other:?}"),
    }

    let state = test.store.get_state();
    assert!(state.error.is_some());
    assert!(state.error.as_ref().unwrap().contains("traceroute failed"));

    tear_down(test.tmp_path);
}

// -- handle_ssh tests --

#[test]
fn handle_ssh_ok_no_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor
        .expect_ssh()
        .returning(|_, _| Ok((ExitStatus::default(), None)));

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let device_config = make_device_config();

    test.main_process
        .handle_ssh(&device, &device_config)
        .unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_none());

    tear_down(test.tmp_path);
}

#[test]
fn handle_ssh_executor_error_sets_store_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor
        .expect_ssh()
        .returning(|_, _| Err(eyre!("ssh connection refused")));

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let device_config = make_device_config();

    test.main_process
        .handle_ssh(&device, &device_config)
        .unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_some());
    assert!(
        state
            .error
            .as_ref()
            .unwrap()
            .contains("ssh connection refused")
    );

    tear_down(test.tmp_path);
}

#[test]
fn handle_ssh_nonzero_exit_with_stderr_sets_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor.expect_ssh().returning(|_, _| {
        let mut tmpfile: File = tempfile::tempfile().unwrap();
        writeln!(tmpfile, "permission denied").unwrap();
        tmpfile.seek(SeekFrom::Start(0)).unwrap();

        let status = ExitStatusExt::from_raw(1);
        let fd = OwnedFd::from(tmpfile);
        let mock_stderr = ChildStderr::from(fd);

        Ok((status, Some(mock_stderr)))
    });

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let device_config = make_device_config();

    test.main_process
        .handle_ssh(&device, &device_config)
        .unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_some());
    assert!(state.error.as_ref().unwrap().contains("permission denied"));

    tear_down(test.tmp_path);
}

#[test]
fn handle_ssh_nonzero_exit_without_stderr_sets_generic_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor.expect_ssh().returning(|_, _| {
        let status = ExitStatusExt::from_raw(1);
        Ok((status, None))
    });

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let device_config = make_device_config();

    test.main_process
        .handle_ssh(&device, &device_config)
        .unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_some());
    assert!(state.error.as_ref().unwrap().contains("ssh command failed"));

    tear_down(test.tmp_path);
}

// -- handle_browse tests --

#[test]
fn handle_browse_ok_no_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor
        .expect_browse()
        .returning(|_| Ok((ExitStatus::default(), None)));

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let args = BrowseArgs {
        device,
        port: 80,
        use_lynx: false,
    };

    test.main_process.handle_browse(&args).unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_none());

    tear_down(test.tmp_path);
}

#[test]
fn handle_browse_executor_error_sets_store_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor
        .expect_browse()
        .returning(|_| Err(eyre!("browser not found")));

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let args = BrowseArgs {
        device,
        port: 80,
        use_lynx: false,
    };

    test.main_process.handle_browse(&args).unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_some());
    assert!(state.error.as_ref().unwrap().contains("browser not found"));

    tear_down(test.tmp_path);
}

#[test]
fn handle_browse_nonzero_exit_with_stderr_sets_error() {
    let mut mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_executor.expect_browse().returning(|_| {
        let mut tmpfile: File = tempfile::tempfile().unwrap();
        writeln!(tmpfile, "connection refused").unwrap();
        tmpfile.seek(SeekFrom::Start(0)).unwrap();

        let status = ExitStatusExt::from_raw(1);
        let fd = OwnedFd::from(tmpfile);
        let mock_stderr = ChildStderr::from(fd);

        Ok((status, Some(mock_stderr)))
    });

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIPaused))
        .times(1);

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::UIResumed))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let device = make_device();
    let args = BrowseArgs {
        device,
        port: 80,
        use_lynx: false,
    };

    test.main_process.handle_browse(&args).unwrap();

    let state = test.store.get_state();
    assert!(state.error.is_some());
    assert!(state.error.as_ref().unwrap().contains("connection refused"));

    tear_down(test.tmp_path);
}

// -- pause_ui / resume_ui tests --

#[test]
fn pause_ui_returns_err_on_quit_message() {
    let mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::PauseUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::Quit(None)))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let result = test.main_process.pause_ui();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("quit received during pause")
    );

    tear_down(test.tmp_path);
}

#[test]
fn resume_ui_returns_err_on_quit_message() {
    let mock_executor = MockShellExecutor::new();
    let mut mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_render_sender
        .expect_send()
        .withf(|msg| matches!(msg, RendererMessage::ResumeUI))
        .returning(|_| Ok(()))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Ok(MainMessage::Quit(Some("error detail".into()))))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let result = test.main_process.resume_ui();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("error detail"));

    tear_down(test.tmp_path);
}

// -- handle_post_action_sync tests --

#[test]
fn handle_post_action_sync_persists_config_update() {
    let mock_executor = MockShellExecutor::new();
    let mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mut mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();

    mock_network_sender
        .expect_send()
        .withf(|msg| matches!(msg, NetworkMessage::ConfigUpdate(_)))
        .returning(|_| Ok(()))
        .times(1);

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let new_config = crate::config::Config::new(
        "new_user".to_string(),
        "/new/identity".to_string(),
        "10.0.0.0/8".to_string(),
    );

    // dispatch the config so it's in the store
    test.store
        .dispatch(Action::UpdateConfig(new_config.clone()));

    let action =
        Box::new(Action::Sync(Box::new(Action::UpdateConfig(new_config))));

    let result = test.main_process.handle_post_action_sync(action);
    assert!(result.is_ok());

    // verify the config was written to the file
    let contents = fs::read_to_string(&test.tmp_path).unwrap();
    assert!(contents.contains("10.0.0.0/8"));

    tear_down(test.tmp_path);
}

#[test]
fn handle_post_action_sync_ignores_non_config_actions() {
    let mock_executor = MockShellExecutor::new();
    let mock_render_sender = MockIpcSender::<RendererMessage>::new();
    let mock_network_sender = MockIpcSender::<NetworkMessage>::new();
    let mock_receiver = MockIpcReceiver::<MainMessage>::new();

    // no expectations on network_sender — it should never
    // be called for non-config actions

    let test = setup(
        mock_executor,
        mock_render_sender,
        mock_network_sender,
        mock_receiver,
    );

    let action = Box::new(Action::Sync(Box::new(Action::SetError(Some(
        "test".into(),
    )))));

    let result = test.main_process.handle_post_action_sync(action);
    assert!(result.is_ok());

    tear_down(test.tmp_path);
}
