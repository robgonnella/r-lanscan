use core::time;
use log::*;
use std::{collections::HashMap, error::Error, io, sync::Arc};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

use super::{
    store::{dispatcher::Dispatcher, types::ViewName},
    views::{config::ConfigView, device::DeviceView, devices::DevicesView},
};

use super::views::View;

struct App {
    dispatcher: Arc<Dispatcher>,
    views: HashMap<ViewName, Box<dyn View>>,
}

impl App {
    fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let mut views: HashMap<ViewName, Box<dyn View>> = HashMap::new();

        views.insert(
            ViewName::Devices,
            Box::new(DevicesView::new(Arc::clone(&dispatcher))),
        );

        views.insert(
            ViewName::Device,
            Box::new(DeviceView::new(Arc::clone(&dispatcher))),
        );

        views.insert(
            ViewName::Config,
            Box::new(ConfigView::new(Arc::clone(&dispatcher))),
        );

        Self { dispatcher, views }
    }
}

pub fn launch(dispatcher: Arc<Dispatcher>) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(dispatcher);
    let res = run_app(&mut terminal, app);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        let state = app.dispatcher.get_state();
        let view = app.views.get_mut(&state.view).unwrap();
        terminal.draw(|f| view.render(f))?;

        // Use poll here so we don't block the thread, this will allow
        // rendering of incoming device data from network as it's received
        if let Ok(has_event) = event::poll(time::Duration::from_secs(1)) {
            if has_event {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        let handled = view.process_key_event(key);
                        if !handled {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}
