//! Main application loop and terminal management.

use color_eyre::eyre::{Context, Result};
use core::time;
use log::*;
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event as CrossTermEvent, KeyCode,
            KeyModifiers,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::Rect,
    prelude::CrosstermBackend,
};
use std::{
    cell::RefCell,
    io::{self, Stdout},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use crate::{
    ipc::message::Message,
    ui::{
        app::{App, Application},
        colors::Theme,
        store::{Dispatcher, Store, action::Action},
        views::traits::CustomWidgetContext,
    },
};

type Backend = CrosstermBackend<Stdout>;

/// Main application coordinating rendering and event handling.
pub struct CrossTermRenderer {
    terminal: RefCell<Terminal<Backend>>,
    // here to enable unit tests - not an ideal solution but okay for now
    test_terminal: Option<Terminal<TestBackend>>,
    store: Arc<Store>,
    app: Box<dyn Application>,
    event_loop_sender: Sender<Message>,
    event_loop_receiver: Receiver<Message>,
}

/// Creates and initializes the terminal application.
pub fn create_renderer(
    tx: Sender<Message>,
    rx: Receiver<Message>,
    theme: Theme,
    store: Arc<Store>,
) -> Result<CrossTermRenderer> {
    // setup terminal
    enable_raw_mode().wrap_err("failed to enter raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .wrap_err("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).wrap_err("failed to create terminal")?;
    Ok(CrossTermRenderer::new(tx, rx, terminal, theme, store))
}

impl CrossTermRenderer {
    fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        terminal: Terminal<Backend>,
        theme: Theme,
        store: Arc<Store>,
    ) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            test_terminal: None,
            store: Arc::clone(&store),
            app: Box::new(App::new(theme, store as Arc<dyn Dispatcher>, tx.clone())),
            event_loop_sender: tx,
            event_loop_receiver: rx,
        }
    }

    // only exposed in tests to enable unit testing App
    // not an ideal solution but okay for now
    #[cfg(test)]
    fn new_test(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        terminal: Terminal<Backend>,
        test_terminal: Terminal<TestBackend>,
        theme: Theme,
        store: Arc<Store>,
    ) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            test_terminal: Some(test_terminal),
            store: Arc::clone(&store),
            app: Box::new(App::new(theme, store, tx.clone())),
            event_loop_sender: tx,
            event_loop_receiver: rx,
        }
    }

    pub fn launch(&self) -> Result<()> {
        self.render()?;
        self.exit()?;
        Ok(())
    }

    fn render(&self) -> Result<()> {
        loop {
            let state = self.store.get_state()?;

            if state.ui_paused {
                if let Ok(evt) = self.event_loop_receiver.recv()
                    && evt == Message::ResumeUI
                {
                    self.restart()?;
                    self.store.dispatch(Action::SetUIPaused(false));
                    self.event_loop_sender.send(Message::UIResumed)?;
                    continue;
                }
            } else if let Ok(evt) = self.event_loop_receiver.try_recv()
                && evt == Message::PauseUI
            {
                self.pause()?;
                self.store.dispatch(Action::SetUIPaused(true));
                self.event_loop_sender.send(Message::UIPaused)?;
                continue;
            }

            let mut ctx = CustomWidgetContext {
                state: &state,
                app_area: Rect::default(),
                ipc: self.event_loop_sender.clone(),
            };

            if self.test_terminal.is_some() {
                // app is under test - just draw once and exit
                // not an ideal solution but okay for now
                let mut terminal = self.test_terminal.clone().unwrap();
                let _ = terminal.draw(|f| {
                    ctx.app_area = f.area();
                    self.app.render_ref(f.area(), f.buffer_mut(), &ctx)
                });
                return Ok(());
            }

            self.terminal.borrow_mut().draw(|f| {
                ctx.app_area = f.area();
                self.app.render_ref(f.area(), f.buffer_mut(), &ctx)
            })?;

            // Use poll here so we don't block the thread, this will allow
            // rendering of incoming device data from network as it's received
            if let Ok(has_event) = event::poll(time::Duration::from_millis(60))
                && has_event
            {
                let evt = event::read()?;

                let handled = self.app.process_event(&evt, &ctx);

                if let CrossTermEvent::Key(key) = evt {
                    match key.code {
                        KeyCode::Char('q') => {
                            // allow overriding q key
                            if !handled {
                                self.event_loop_sender.send(Message::Quit)?;
                                return Ok(());
                            }
                        }
                        KeyCode::Char('c') => {
                            // do not allow overriding ctrl-c
                            if key.modifiers == KeyModifiers::CONTROL {
                                info!("APP RECEIVED CONTROL-C SEQUENCE");
                                self.event_loop_sender.send(Message::Quit)?;
                                return Ok(());
                            }
                        }
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
        if self.test_terminal.is_none() {
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
        }
        Ok(())
    }

    fn exit(&self) -> Result<()> {
        if self.test_terminal.is_none() {
            let mut terminal = self.terminal.borrow_mut();
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "./cross_term_tests.rs"]
mod tests;
