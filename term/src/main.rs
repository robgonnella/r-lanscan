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
    fs, io,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender, channel},
    },
    thread::{self, JoinHandle},
};

use ui::store::{Store, action::Action};

use crate::{
    config::DEFAULT_CONFIG_ID,
    ipc::{
        main::{MainIpc, MainReceiver, MainSender},
        message::{MainMessage, RendererMessage},
        renderer::{RendererIpc, RendererReceiver, RendererSender},
    },
    renderer::process::Renderer,
    shell::Shell,
    ui::{
        colors::Theme,
        store::{Dispatcher, StateGetter, SubscriptionProvider, state::State},
    },
};

#[doc(hidden)]
mod config;
#[doc(hidden)]
mod error;
#[doc(hidden)]
mod ipc;
#[doc(hidden)]
mod main_event_handler;
#[doc(hidden)]
mod network;
#[doc(hidden)]
mod renderer;
#[doc(hidden)]
mod shell;
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
    store: Arc<Store>,
) -> Result<()> {
    let (_, exit_rx) = mpsc::channel();
    let wire = packet::wire::default(&interface)?;

    // ignore handle here - we will forcefully exit instead of waiting
    // for scan to finish
    thread::spawn(move || -> Result<()> {
        network::monitor_network(
            exit_rx, wire.0, wire.1, config, interface, store,
        )
    });

    Ok(())
}

#[doc(hidden)]
fn start_renderer_thread(
    main_tx: Sender<MainMessage>,
    renderer_rx: Receiver<RendererMessage>,
    store: Arc<Store>,
    initial_theme: Theme,
) -> JoinHandle<Result<()>> {
    // start tui renderer thread
    thread::spawn(move || {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let renderer_ipc = RendererIpc::new(
            Box::new(RendererSender::new(main_tx)),
            Box::new(RendererReceiver::new(renderer_rx)),
        );
        let app_renderer =
            Renderer::new(terminal, store, renderer_ipc, initial_theme);
        app_renderer.start_render_loop()
    })
}

#[doc(hidden)]
fn process_main_thread_events(
    renderer_tx: Sender<RendererMessage>,
    main_rx: Receiver<MainMessage>,
    store: Arc<Store>,
) -> Result<()> {
    let main_ipc = MainIpc::new(
        Box::new(MainSender::new(renderer_tx)),
        Box::new(MainReceiver::new(main_rx)),
    );

    let handler = main_event_handler::MainEventHandler::new(
        store,
        Box::new(Shell::new()),
        main_ipc,
    );

    // block and process incoming ipc events
    handler.process_events()
}

#[doc(hidden)]
fn register_state_listener(
    renderer_tx: Sender<RendererMessage>,
    store: &mut Store,
) {
    store.subscribe(move |_: &State| {
        renderer_tx.send(RendererMessage::ReRender).map_err(|e| {
            eyre!("failed to send rerender message to renderer process: {}", e)
        })
    });
}

#[doc(hidden)]
fn init(args: &Args, interface: &NetworkInterface) -> Result<(Config, Store)> {
    let (user, identity) = get_user_info()?;

    let config_manager =
        create_config_manager(user.clone(), identity.clone(), interface)?;

    let (current_config, should_create_config) = get_current_config(
        &config_manager,
        interface,
        args.ports.clone(),
        user,
        identity,
    );

    let current_config_id = current_config.id.clone();

    let store = Store::new(
        Arc::new(Mutex::new(config_manager)),
        current_config.clone(),
    );

    if should_create_config {
        store.dispatch(Action::CreateAndSetConfig(current_config.clone()))?;
    } else {
        store.load_config(&current_config_id)?;
    }

    Ok((current_config, store))
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
    let (config, mut store) = init(&args, &interface)?;
    let initial_theme = store.get_state()?.theme;

    let (renderer_tx, renderer_rx) = channel();
    let (main_tx, main_rx) = channel();

    register_state_listener(renderer_tx.clone(), &mut store);

    let store = Arc::new(store);

    // ignore handle here - we will forcefully exit instead of waiting
    // for scan to finish
    start_network_monitoring_thread(
        config,
        Arc::new(interface),
        Arc::clone(&store),
    )?;

    if args.debug {
        let mut signals = Signals::new([SIGINT])?;
        let _ = signals.wait();
        return Ok(());
    }

    // start separate thread for tui rendering process
    let renderer_handle = start_renderer_thread(
        main_tx,
        renderer_rx,
        Arc::clone(&store),
        initial_theme,
    );

    // loop / block and process incoming ipc messages in main thread
    process_main_thread_events(renderer_tx, main_rx, Arc::clone(&store))?;

    // wait for renderer thread to exit
    renderer_handle
        .join()
        .map_err(error::report_from_thread_panic)?
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
