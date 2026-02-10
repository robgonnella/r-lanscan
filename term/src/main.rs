//! Terminal UI (TUI) app for managing networked LAN devices
//!
//! This is the new and improved rust version of [ops](https://github.com/robgonnella/ops)
//!
//! # Features:
//!
//! - Save global ssh configuration used for all devices
//! - Save device specific ssh config that overrides global configuration
//! - Drop to ssh for any device found on network (requires ssh client to be installed)
//! - Run `traceroute` for device found on network (requires traceroute to be installed)
//! - Open terminal web browser on any port for any device found on network (requires lynx browser to be installed)
//!
//! # Examples
//!
//! ```bash
//! # show help menu
//! sudo r-lanterm --help
//!
//! # launch application
//! sudo r-lanterm
//! ```

use clap::Parser;
use color_eyre::eyre::{ContextCompat, Result, eyre};
use config::{Config, ConfigManager};
use directories::ProjectDirs;
use r_lanlib::{
    network::{NetworkInterface, get_default_interface},
    packet,
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::{
    cell::RefCell,
    fs, io,
    rc::Rc,
    sync::{
        Arc,
        mpsc::{Receiver, Sender, channel},
    },
    thread::{self, JoinHandle},
};

use crate::{
    config::DEFAULT_CONFIG_ID,
    ipc::{
        main::{MainIpc, MainReceiver, MainSender},
        message::{MainMessage, NetworkMessage, RendererMessage},
        network::{NetworkIpc, NetworkReceiver, NetworkSender},
        renderer::{RendererIpc, RendererReceiver, RendererSender},
    },
    process::{
        main::process::MainProcess,
        network::{process::NetworkProcess, traits::NetworkMonitor},
        renderer::process::RendererProcess,
    },
    shell::Shell,
    store::{Store, action::Action, reducer::StoreReducer, state::State},
    ui::colors::{Colors, Theme},
};

#[doc(hidden)]
mod config;
#[doc(hidden)]
mod error;
#[doc(hidden)]
mod ipc;
#[doc(hidden)]
mod process;
#[doc(hidden)]
mod shell;
#[doc(hidden)]
mod store;
#[doc(hidden)]
mod ui;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in debug mode - Only prints logs foregoing UI
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    /// Comma separated list of ports and port ranges to scan
    #[arg(
        short,
        long,
        default_value = config::DEFAULT_PORTS_STR,
        use_value_delimiter = true
    )]
    ports: Vec<String>,
}

#[doc(hidden)]
fn initialize_logger(args: &Args) -> Result<()> {
    let filter = if args.debug {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Off
    };

    simplelog::TermLogger::init(
        filter,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    Ok(())
}

#[doc(hidden)]
fn get_project_config_path() -> Result<String> {
    let project_dir = ProjectDirs::from("", "", "r-lanterm")
        .ok_or(eyre!("failed to get project directory"))?;
    let config_dir = project_dir.config_dir();
    fs::create_dir_all(config_dir)?;
    let config_file_path = config_dir
        .join("config.yml")
        .to_str()
        .ok_or(eyre!("unable to construct config file path"))?
        .to_string();
    Ok(config_file_path)
}

#[doc(hidden)]
fn get_user_info() -> Result<(String, String)> {
    let user = whoami::username()?;
    let home = dirs::home_dir()
        .wrap_err(eyre!("failed to get user's home directory"))?;
    let identity = format!("{}/.ssh/id_rsa", home.to_string_lossy());
    Ok((user, identity))
}

#[doc(hidden)]
fn create_config_manager(
    user: String,
    identity: String,
    interface: &NetworkInterface,
) -> Result<ConfigManager> {
    let config_path = get_project_config_path()?;

    let config_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(interface.cidr.clone())
        .path(config_path)
        .build()?;

    Ok(config_manager)
}

#[doc(hidden)]
fn get_current_config(
    config_manager: &ConfigManager,
    interface: &NetworkInterface,
    ports: Vec<String>,
    user: String,
    identity: String,
) -> (Config, bool) {
    config_manager
        .get_by_cidr(&interface.cidr)
        .or_else(|| config_manager.get_by_id(DEFAULT_CONFIG_ID))
        .map(|c| (c, false))
        .unwrap_or_else(|| {
            (
                Config {
                    id: fakeit::animal::animal().to_lowercase(),
                    cidr: interface.cidr.clone(),
                    ports,
                    ..Config::new(user, identity, interface.cidr.clone())
                },
                true,
            )
        })
}

#[doc(hidden)]
fn start_network_monitoring_thread(
    config: Config,
    interface: Arc<NetworkInterface>,
    tx: Sender<MainMessage>,
    rx: Receiver<NetworkMessage>,
) -> Result<JoinHandle<Result<()>>> {
    let wire = packet::wire::default(&interface)?;

    let ipc = NetworkIpc::new(
        Box::new(NetworkSender::new(tx)),
        Box::new(NetworkReceiver::new(rx)),
    );

    let network_process = NetworkProcess::builder()
        .interface(interface)
        .wire(wire)
        .ipc(ipc)
        .config(RefCell::new(config))
        .build()?;

    Ok(thread::spawn(move || -> Result<()> {
        network_process.monitor()
    }))
}

#[doc(hidden)]
fn start_renderer_thread(
    main_tx: Sender<MainMessage>,
    renderer_rx: Receiver<RendererMessage>,
    initial_state: State,
) -> JoinHandle<Result<()>> {
    // start tui renderer thread
    thread::spawn(move || {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let main_tx_clone = main_tx.clone();
        let renderer_ipc = RendererIpc::new(
            Box::new(RendererSender::new(main_tx)),
            Box::new(RendererReceiver::new(renderer_rx)),
        );
        let mut store = Store::new(initial_state, StoreReducer::boxed());
        store.set_sync_fn(move |a| {
            let _ = main_tx_clone.send(MainMessage::ActionSync(Box::new(
                Action::Sync(Box::new(a)),
            )));
        });
        let renderer_process =
            RendererProcess::new(terminal, renderer_ipc, Rc::new(store));
        renderer_process.start_render_loop()
    })
}

#[doc(hidden)]
fn start_main_process(
    config_manager: ConfigManager,
    renderer_tx: Sender<RendererMessage>,
    network_tx: Sender<NetworkMessage>,
    main_rx: Receiver<MainMessage>,
    initial_state: State,
) -> Result<()> {
    let renderer_tx_clone = renderer_tx.clone();
    let main_ipc = MainIpc::new(
        Box::new(MainSender::new(renderer_tx.clone(), network_tx.clone())),
        Box::new(MainSender::new(renderer_tx, network_tx)),
        Box::new(MainReceiver::new(main_rx)),
    );

    let mut store = Store::new(initial_state, StoreReducer::boxed());

    store.set_sync_fn(move |a| {
        let _ = renderer_tx_clone.send(RendererMessage::ActionSync(Box::new(
            Action::Sync(Box::new(a)),
        )));
    });

    let main_process = MainProcess::builder()
        .config_manager(RefCell::new(config_manager))
        .executor(Box::new(Shell::new()))
        .store(Rc::new(store))
        .ipc(main_ipc)
        .build()?;

    // block and process incoming ipc` events
    main_process.process_events()
}

#[doc(hidden)]
fn init(
    args: &Args,
    interface: &NetworkInterface,
) -> Result<(ConfigManager, State)> {
    let (user, identity) = get_user_info()?;

    let mut config_manager =
        create_config_manager(user.clone(), identity.clone(), interface)?;

    let (current_config, should_create_config) = get_current_config(
        &config_manager,
        interface,
        args.ports.clone(),
        user,
        identity,
    );

    if should_create_config {
        config_manager.create(&current_config)?;
    }

    let true_color_enabled =
        match supports_color::on(supports_color::Stream::Stdout) {
            Some(support) => support.has_16m,
            _ => false,
        };

    let theme = Theme::from_string(&current_config.theme);

    let colors =
        Colors::new(theme.to_palette(true_color_enabled), true_color_enabled);

    let initial_state = State {
        true_color_enabled,
        config: current_config.clone(),
        theme: Theme::from_string(&current_config.theme),
        colors,
        ..Default::default()
    };

    Ok((config_manager, initial_state))
}

#[doc(hidden)]
fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

#[doc(hidden)]
fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    initialize_logger(&args)?;

    if !is_root() {
        return Err(eyre!("permission denied: must run with root privileges"));
    }

    let interface = get_default_interface()
        .ok_or_else(|| eyre!("Could not detect default network interface"))?;

    let (config_manager, initial_state) = init(&args, &interface)?;

    let (renderer_tx, renderer_rx) = channel();
    let (main_tx, main_rx) = channel();
    let (network_tx, network_rx) = channel();

    // start network monitoring thread
    // no need to get handle and join - when main process exits
    // everything should go with it including network monitoring thread
    start_network_monitoring_thread(
        initial_state.config.clone(),
        Arc::new(interface),
        main_tx.clone(),
        network_rx,
    )?;

    if args.debug {
        let mut signals = Signals::new([SIGINT])?;
        let _ = signals.wait();
        return Ok(());
    }

    // start separate thread for tui rendering process
    // no need to get handle and join - when main process exits
    // everything should go with it including renderer thread
    start_renderer_thread(main_tx, renderer_rx, initial_state.clone());

    // loop / block and process incoming ipc messages in main thread
    start_main_process(
        config_manager,
        renderer_tx,
        network_tx,
        main_rx,
        initial_state,
    )
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
