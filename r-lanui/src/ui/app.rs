use color_eyre::eyre::Report;
use core::time;
use log::*;
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
    io::{self, Stdout},
    process::Command,
    sync::Arc,
};

use super::{
    store::{action::Action, state::State, store::Store},
    views::{main::MainView, View},
};

struct App {
    store: Arc<Store>,
    main_view: Box<dyn View>,
}

impl App {
    fn new(store: Arc<Store>) -> Self {
        let store_clone = Arc::clone(&store);
        Self {
            store,
            main_view: Box::new(MainView::new(store_clone)),
        }
    }

    pub fn get_state(&self) -> State {
        self.store.get_state()
    }
}

pub fn launch(store: Arc<Store>) -> Result<(), Report> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(store);

    // start app loop
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    let mut paused = false;

    loop {
        let state = app.get_state();

        if paused && !state.paused {
            // unpause
            enable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )?;
            terminal.hide_cursor()?;
            terminal.clear()?;

            paused = false;
        }

        if paused {
            continue;
        }

        if state.paused && !paused {
            // pause - restore terminal to user
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            paused = true;

            let mut handle = Command::new("sh")
                .spawn()
                .expect("failed to start shell command");
            handle.wait().expect("shell command failed");
            app.store.dispatch(Action::TogglePause);
            continue;
        }

        terminal.draw(|f| app.main_view.render_ref(f.area(), f.buffer_mut()))?;

        // Use poll here so we don't block the thread, this will allow
        // rendering of incoming device data from network as it's received
        if let Ok(has_event) = event::poll(time::Duration::from_millis(60)) {
            if has_event {
                let evt = event::read()?;

                let handled = app.main_view.process_event(&evt, &state);

                if !handled {
                    match evt {
                        Event::Key(key) => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('c') => {
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
}
