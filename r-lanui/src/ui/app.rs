use core::time;
use log::*;
use std::{collections::HashMap, error::Error, io, sync::Arc};

use ratatui::{
    backend::Backend,
    crossterm::event::{self, Event, KeyCode},
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
    color_eyre::install()?;

    // setup terminal
    let mut terminal = ratatui::init();
    let app = App::new(dispatcher);

    // start app loop
    let res = run_app(&mut terminal, app);

    // restore terminal
    ratatui::restore();

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
                let evt = event::read()?;
                let handled = view.process_event(&evt);
                if !handled {
                    match evt {
                        Event::Key(key) => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            _ => {}
                        },
                        Event::FocusGained => {}
                        Event::FocusLost => {}
                        Event::Mouse(_m) => {}
                        Event::Paste(_p) => {}
                        Event::Resize(_x, _y) => {}
                    }
                }
            }
        }
    }
}