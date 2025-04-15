use color_eyre::eyre::{Context, Result};
use core::time;
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::CrosstermBackend,
    Terminal,
};
use std::{
    cell::RefCell,
    io::{self, Stdout},
    process::Command,
    sync::Arc,
};

use super::{
    store::{action::Action, state::Command as AppCommand, store::Store},
    views::{main::MainView, traits::View},
};

pub struct App {
    terminal: RefCell<Terminal<CrosstermBackend<Stdout>>>,
    paused: RefCell<bool>,
    store: Arc<Store>,
    main_view: Box<dyn View>,
}

pub fn create_app(store: Arc<Store>) -> Result<App> {
    // setup terminal
    enable_raw_mode().wrap_err("failed to enter raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .wrap_err("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).wrap_err("failed to create terminal")?;
    Ok(App::new(terminal, store))
}

impl App {
    fn new(terminal: Terminal<CrosstermBackend<Stdout>>, store: Arc<Store>) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            paused: RefCell::new(false),
            store: Arc::clone(&store),
            main_view: Box::new(MainView::new(store)),
        }
    }

    pub fn launch(&self) -> Result<()> {
        self.process_loop()?;
        self.exit()?;
        Ok(())
    }

    fn process_loop(&self) -> Result<()> {
        loop {
            let state = self.store.get_state();

            if *self.paused.borrow() && state.execute_cmd.is_none() {
                // unpause
                self.restart()?;
                *self.paused.borrow_mut() = false;
            }

            if *self.paused.borrow() {
                continue;
            }

            if state.execute_cmd.is_some() && !*self.paused.borrow() {
                let cmd = state.execute_cmd.clone().unwrap();
                match cmd {
                    AppCommand::SSH => {
                        if state.selected_device.is_some() && state.selected_device_config.is_some()
                        {
                            let device = state.selected_device.unwrap();
                            let device_config = state.selected_device_config.unwrap();
                            self.pause()?;
                            let mut handle = Command::new("ssh")
                                .arg("-i")
                                .arg(device_config.ssh_identity_file)
                                .arg(format!("{}@{}", device_config.ssh_user, device.ip))
                                .arg("-p")
                                .arg(device_config.ssh_port.to_string())
                                .spawn()
                                .wrap_err("failed to start ssh shell command")?;
                            handle.wait().wrap_err("shell command failed")?;
                            self.store.dispatch(Action::ClearCommand);
                            continue;
                        }
                    }
                }
            }

            self.terminal
                .borrow_mut()
                .draw(|f| self.main_view.render_ref(f.area(), f.buffer_mut()))?;

            // Use poll here so we don't block the thread, this will allow
            // rendering of incoming device data from network as it's received
            if let Ok(has_event) = event::poll(time::Duration::from_millis(60)) {
                if has_event {
                    let evt = event::read()?;

                    let handled = self.main_view.process_event(&evt, &state);

                    match evt {
                        Event::Key(key) => match key.code {
                            KeyCode::Char('q') => {
                                // allow overriding q key
                                if !handled {
                                    return Ok(());
                                }
                            }
                            KeyCode::Char('c') => {
                                // do not allow overriding ctrl-c
                                if key.modifiers == KeyModifiers::CONTROL {
                                    return Ok(());
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    fn pause(&self) -> Result<()> {
        self.exit()?;
        *self.paused.borrow_mut() = true;
        Ok(())
    }

    fn restart(&self) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();
        enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        terminal.hide_cursor()?;
        terminal.clear()?;
        *self.paused.borrow_mut() = false;
        Ok(())
    }

    fn exit(&self) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }
}
