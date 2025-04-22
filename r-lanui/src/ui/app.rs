use color_eyre::eyre::{Context, Result};
use core::time;
use log::*;
use ratatui::{
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event as CrossTermEvent, KeyCode,
            KeyModifiers,
        },
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::Rect,
    prelude::CrosstermBackend,
    Terminal,
};
use std::{
    cell::RefCell,
    io::{self, Stdout},
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use super::{
    events::types::Event,
    store::{action::Action, store::Store},
    views::{
        main::MainView,
        traits::{CustomWidgetContext, View},
    },
};

pub struct App {
    terminal: RefCell<Terminal<CrosstermBackend<Stdout>>>,
    store: Arc<Store>,
    main_view: Box<dyn View>,
    event_loop_sender: Sender<Event>,
    event_loop_receiver: Receiver<Event>,
}

pub fn create_app(tx: Sender<Event>, rx: Receiver<Event>, store: Arc<Store>) -> Result<App> {
    // setup terminal
    enable_raw_mode().wrap_err("failed to enter raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .wrap_err("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).wrap_err("failed to create terminal")?;
    Ok(App::new(tx, rx, terminal, store))
}

impl App {
    fn new(
        tx: Sender<Event>,
        rx: Receiver<Event>,
        terminal: Terminal<CrosstermBackend<Stdout>>,
        store: Arc<Store>,
    ) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            store: Arc::clone(&store),
            main_view: Box::new(MainView::new(store, tx.clone())),
            event_loop_sender: tx,
            event_loop_receiver: rx,
        }
    }

    pub fn launch(&self) -> Result<()> {
        self.start_app_loop()?;
        self.exit()?;
        Ok(())
    }

    fn start_app_loop(&self) -> Result<()> {
        loop {
            let state = self.store.get_state();

            if state.ui_paused {
                if let Ok(evt) = self.event_loop_receiver.recv() {
                    match evt {
                        Event::ResumeUI => {
                            self.restart()?;
                            self.store.dispatch(Action::SetUIPaused(false));
                            self.event_loop_sender.send(Event::UIResumed)?;
                            continue;
                        }
                        _ => {}
                    }
                }
            } else if let Ok(evt) = self.event_loop_receiver.try_recv() {
                match evt {
                    Event::PauseUI => {
                        self.pause()?;
                        self.store.dispatch(Action::SetUIPaused(true));
                        self.event_loop_sender.send(Event::UIPaused)?;
                        continue;
                    }
                    _ => {}
                }
            }

            let mut ctx = CustomWidgetContext {
                state: state.clone(),
                app_area: Rect::default(),
                events: self.event_loop_sender.clone(),
            };

            self.terminal.borrow_mut().draw(|f| {
                ctx = CustomWidgetContext {
                    state,
                    app_area: f.area().clone(),
                    events: self.event_loop_sender.clone(),
                };
                self.main_view.render_ref(f.area(), f.buffer_mut(), &ctx)
            })?;

            // Use poll here so we don't block the thread, this will allow
            // rendering of incoming device data from network as it's received
            if let Ok(has_event) = event::poll(time::Duration::from_millis(60)) {
                if has_event {
                    let evt = event::read()?;

                    let handled = self.main_view.process_event(&evt, &ctx);

                    match evt {
                        CrossTermEvent::Key(key) => match key.code {
                            KeyCode::Char('q') => {
                                // allow overriding q key
                                if !handled {
                                    self.event_loop_sender.send(Event::Quit)?;
                                    return Ok(());
                                }
                            }
                            KeyCode::Char('c') => {
                                // do not allow overriding ctrl-c
                                if key.modifiers == KeyModifiers::CONTROL {
                                    info!("APP RECEIVED CONTROL-C SEQUENCE");
                                    self.event_loop_sender.send(Event::Quit)?;
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
        self.store.dispatch(Action::SetUIPaused(true));
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
        self.store.dispatch(Action::SetUIPaused(false));
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
