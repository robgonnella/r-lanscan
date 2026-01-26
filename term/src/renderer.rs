//! Main application loop and terminal management.

use color_eyre::eyre::{Context, Result};
use core::time;
use log::*;
use ratatui::{
    Terminal,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event as CrossTermEvent, KeyCode,
            KeyModifiers,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
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
pub struct Renderer<B: Backend + std::io::Write> {
    terminal: RefCell<Terminal<B>>,
    store: Arc<Store>,
    app: Box<dyn Application>,
    ipc: RendererIpc,
}

impl<B: Backend + std::io::Write> Renderer<B> {
    pub fn new(terminal: Terminal<B>, theme: Theme, store: Arc<Store>, ipc: RendererIpc) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            store: Arc::clone(&store),
            app: Box::new(App::new(theme, store as Arc<dyn Dispatcher>)),
            ipc,
        }
    }

    pub fn launch(&self) -> Result<()> {
        enable_raw_mode().wrap_err("failed to enter raw mode")?;
        // Note we must use io::stdout() directly here. Using
        // self.terminal.borrow_mut().backend_mut() will result in immediate
        // exit. I believe this is maybe due to backend being dropped after the
        // call to execute?
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
            .wrap_err("failed to enter alternate screen")?;
        self.render()?;
        self.exit()?;
        Ok(())
    }

    fn render(&self) -> Result<()> {
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
                                self.ipc.tx.send(MainMessage::Quit)?;
                                return Ok(());
                            }
                        }
                        KeyCode::Char('c') => {
                            // do not allow overriding ctrl-c
                            if key.modifiers == KeyModifiers::CONTROL {
                                info!("APP RECEIVED CONTROL-C SEQUENCE");
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
