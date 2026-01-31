//! Main application loop and terminal management.

use color_eyre::eyre::{Context, Result, eyre};
use core::time;
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, Event as CrossTermEvent, KeyCode, KeyModifiers},
        execute,
        terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
            enable_raw_mode,
        },
    },
    layout::Rect,
    prelude::Backend,
};
use std::{cell::RefCell, io, sync::Arc};

use crate::{
    ipc::{
        message::{MainMessage, RendererMessage},
        renderer::RendererIpc,
    },
    ui::{
        app::{App, Application},
        colors::Theme,
        store::{Dispatcher, Store, action::Action},
        views::traits::CustomWidgetContext,
    },
};

/// Main application coordinating rendering and event handling.
///
/// Manages the terminal lifecycle (raw mode, alternate screen) and runs the
/// render loop that draws UI and processes input events.
pub struct Renderer<B: Backend + std::io::Write> {
    terminal: RefCell<Terminal<B>>,
    store: Arc<Store>,
    app: Box<dyn Application>,
    ipc: RendererIpc,
}

impl<B: Backend + std::io::Write> Renderer<B> {
    /// Creates a new renderer with the given terminal, theme, store, and IPC.
    pub fn new(
        terminal: Terminal<B>,
        theme: Theme,
        store: Arc<Store>,
        ipc: RendererIpc,
    ) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            store: Arc::clone(&store),
            app: Box::new(App::new(theme, store as Arc<dyn Dispatcher>)),
            ipc,
        }
    }

    /// Initializes the terminal and starts the render loop. Returns when the
    /// user quits.
    pub fn start_render_loop(&self) -> Result<()> {
        self.enable_terminal_raw_mode()?;
        self.start_loop()?;
        self.exit()
    }

    fn start_loop(&self) -> Result<()> {
        loop {
            let state = self.store.get_state()?;

            if state.ui_paused {
                if let Ok(evt) = self.ipc.rx.recv()
                    && evt == RendererMessage::ResumeUI
                {
                    self.restart()?;
                    self.ipc.tx.send(MainMessage::UIResumed)?;
                    continue;
                }
            } else if let Ok(evt) = self.ipc.rx.try_recv()
                && evt == RendererMessage::PauseUI
            {
                self.pause()?;
                self.ipc.tx.send(MainMessage::UIPaused)?;
                continue;
            }

            let mut ctx = CustomWidgetContext {
                state: &state,
                app_area: Rect::default(),
                ipc: self.ipc.tx.clone(),
            };

            self.terminal
                .borrow_mut()
                .draw(|f| {
                    ctx.app_area = f.area();
                    self.app.render_ref(f.area(), f.buffer_mut(), &ctx)
                })
                .map_err(|e| eyre!("failed to render: {}", e))?;

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
                                self.ipc.tx.send(MainMessage::Quit)?;
                                return Ok(());
                            }
                        }
                        KeyCode::Char('c') => {
                            // do not allow overriding ctrl-c
                            if key.modifiers == KeyModifiers::CONTROL {
                                self.store.dispatch(Action::Log(
                                    "APP RECEIVED CONTROL-C SEQUENCE".into(),
                                ));
                                self.ipc.tx.send(MainMessage::Quit)?;
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn enable_terminal_raw_mode(&self) -> Result<()> {
        enable_raw_mode().wrap_err("failed to enter raw mode")?;
        // Note we must use io::stdout() directly here. Using
        // self.terminal.borrow_mut().backend_mut() will result in immediate
        // exit. I believe this is due to mutable borrow of backend being
        // quickly dropped after the call to execute?
        execute!(io::stdout(), EnterAlternateScreen)
            .wrap_err("failed to enter alternate screen")?;
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        self.exit()?;
        self.store.dispatch(Action::SetUIPaused(true));
        Ok(())
    }

    fn restart(&self) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        terminal
            .hide_cursor()
            .map_err(|e| eyre!("failed to hide terminal cursor: {}", e))?;
        terminal
            .clear()
            .map_err(|e| eyre!("failed to clear terminal: {}", e))?;
        self.store.dispatch(Action::SetUIPaused(false));
        Ok(())
    }

    fn exit(&self) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal
            .show_cursor()
            .map_err(|e| eyre!("failed to show terminal cursor: {}", e))?;
        Ok(())
    }
}
