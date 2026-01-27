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
        mpsc::{self, channel},
    },
    thread,
};

use ui::store::{Store, action::Action};

use crate::{
    config::DEFAULT_CONFIG_ID,
    ipc::{
        main::{MainIpc, MainReceiver, MainSender},
        renderer::{RendererIpc, RendererReceiver, RendererSender},
    },
    shell::Shell,
    ui::{colors::Theme, store::Dispatcher},
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
fn init(
    args: &Args,
    interface: &NetworkInterface,
) -> Result<(Config, Arc<Store>)> {
    let user = whoami::username()?;
    let home = dirs::home_dir()
        .wrap_err(eyre!("failed to get user's home directory"))?;
    let identity = format!("{}/.ssh/id_rsa", home.to_string_lossy());

    let config_path = get_project_config_path()?;

    let config_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(interface.cidr.clone())
        .path(config_path)
        .build()?;

    let config_manager = Arc::new(Mutex::new(config_manager));

    let manager = config_manager
        .lock()
        .map_err(|e| eyre!("failed to get lock on config_manager: {}", e))?;

    let mut create_config = false;

    let current_config = manager
        .get_by_cidr(&interface.cidr)
        .or_else(|| manager.get_by_id(DEFAULT_CONFIG_ID))
        .unwrap_or_else(|| {
            create_config = true;
            Config {
                id: fakeit::animal::animal().to_lowercase(),
                cidr: interface.cidr.clone(),
                ports: args.ports.clone(),
                ..Config::new(
                    user.clone(),
                    identity.clone(),
                    interface.cidr.clone(),
                )
            }
        });

    let current_config_id = current_config.id.clone();

    // free up manager lock so dispatches can acquire lock as needed
    drop(manager);

    let store = Arc::new(Store::new(
        Arc::clone(&config_manager),
        current_config.clone(),
    ));

    if create_config {
        store.dispatch(Action::CreateAndSetConfig(current_config.clone()));
    } else {
        store.dispatch(Action::SetConfig(current_config_id));
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
    let (config, store) = init(&args, &interface)?;
    let theme = Theme::from_string(&config.theme);
    let (_, exit_rx) = mpsc::channel();
    let wire = packet::wire::default(&interface)?;
    let net_monitor_store = Arc::clone(&store);

    // ignore handle here - we will forcefully exit instead of waiting
    // for scan to finish
    thread::spawn(move || -> Result<()> {
        network::monitor_network(
            exit_rx,
            wire.0,
            wire.1,
            config,
            Arc::new(interface),
            net_monitor_store,
        )
    });

    if args.debug {
        let mut signals = Signals::new([SIGINT])?;
        let _ = signals.wait();
        return Ok(());
    }

    // captures ctrl-c only in main thread so when we drop down to shell
    // commands like ssh, we will pause the key handler for ctrl-c in app
    // and capture ctrl-c here to prevent exiting app and just let ctrl-c
    // be handled by the command being executed, which should return us
    // to our app where we can restart our ui and key-handlers
    ctrlc::set_handler(move || println!("captured ctrl-c!"))?;

    let (renderer_tx, renderer_rx) = channel();
    let (main_tx, main_rx) = channel();

    let renderer_store = Arc::clone(&store);

    // start tui renderer thread
    let renderer_handle = thread::spawn(move || {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let renderer_ipc = RendererIpc::new(
            Box::new(RendererSender::new(main_tx)),
            Box::new(RendererReceiver::new(renderer_rx)),
        );
        let app_renderer = renderer::Renderer::new(
            terminal,
            theme,
            renderer_store,
            renderer_ipc,
        );
        app_renderer.start_render_loop()
    });

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
    handler.process_events()?;

    // wait for renderer thread to exit
    renderer_handle
        .join()
        .map_err(error::report_from_thread_panic)?
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
