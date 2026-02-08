//! Main application loop and terminal management.

use color_eyre::eyre::{Context, Result, eyre};
use core::time;
use ratatui::{
    Terminal,
    crossterm::{
        event::{
            self, Event as CrossTermEvent, KeyCode, KeyEventKind, KeyModifiers,
        },
        execute,
        terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
            enable_raw_mode,
        },
    },
    prelude::Backend,
};
use std::{cell::RefCell, io, sync::Arc};

use crate::{
    ipc::{
        message::{MainMessage, RendererMessage},
        renderer::RendererIpc,
    },
    renderer::scroll_throttle::ScrollThrottle,
    ui::{
        app::{App, Application},
        colors::Theme,
        store::{Dispatcher, StateGetter, Store, action::Action, state::State},
        views::traits::{CustomEventContext, CustomWidgetContext},
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
    scroll_throttle: ScrollThrottle,
}

impl<B: Backend + std::io::Write> Renderer<B> {
    /// Creates a new renderer with the given terminal, theme, store, and IPC.
    pub fn new(
        terminal: Terminal<B>,
        store: Arc<Store>,
        ipc: RendererIpc,
        initial_theme: Theme,
    ) -> Self {
        Self {
            terminal: RefCell::new(terminal),
            store: Arc::clone(&store),
            app: Box::new(App::new(
                store as Arc<dyn Dispatcher>,
                initial_theme,
            )),
            ipc,
            scroll_throttle: ScrollThrottle::default(),
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
        // render initial frame
        let state = self.store.get_state()?;
        self.render_frame(&state)?;

        // now start event handling loop
        loop {
            let state = self.store.get_state()?;

            // When paused, block on recv() to avoid busy-spinning.
            // No rendering or event polling needed while paused.
            if state.ui_paused {
                match self.ipc.rx.recv() {
                    Ok(RendererMessage::ResumeUI) => {
                        self.restart()?;
                    }
                    Ok(_) => {
                        // Ignore PauseUI (duplicate) and
                        // ReRender (can't render while paused)
                    }
                    Err(_) => {
                        // Channel closed; exit loop
                        return self.exit();
                    }
                }
                continue;
            }

            // Use try_recv so we don't block the thread, allowing rendering
            // of incoming device data as it's received
            if let Ok(ipc_msg) = self.ipc.rx.try_recv() {
                match ipc_msg {
                    RendererMessage::PauseUI => self.pause()?,
                    RendererMessage::ResumeUI => self.restart()?,
                    RendererMessage::ReRender => self.render_frame(&state)?,
                }
            }

            // Use poll so we don't block the thread, allowing rendering of
            // incoming device data as it's received
            if let Ok(has_event) = event::poll(time::Duration::from_millis(16))
                && has_event
            {
                let evt = event::read()?;

                if self.scroll_throttle.throttled(&evt) {
                    continue;
                }

                let ctx = CustomEventContext {
                    state: &state,
                    ipc: self.ipc.tx.clone(),
                };

                // Process event through the application. We don't check the
                // return value (whether event was handled) since we removed
                // the 'q' key quit override. All quit operations now happen
                // explicitly via ctrl-c or from within specific views.
                self.app.process_event(&evt, &ctx)?;

                // do not allow overriding ctrl-c
                if let CrossTermEvent::Key(key) = evt
                    && key.kind == KeyEventKind::Press
                    && key.code == KeyCode::Char('c')
                    && key.modifiers == KeyModifiers::CONTROL
                {
                    self.ipc.tx.send(MainMessage::Quit(None))?;
                    return Ok(());
                }

                self.render_frame(&state)?;
            }
        }
    }

    fn render_frame(&self, state: &State) -> Result<()> {
        if state.ui_paused {
            return Ok(());
        }

        let mut res = Ok(());

        self.terminal
            .borrow_mut()
            .draw(|f| {
                let ctx = CustomWidgetContext {
                    state,
                    app_area: f.area(),
                };

                if let Err(err) =
                    self.app.render_ref(f.area(), f.buffer_mut(), &ctx)
                {
                    res = Err(err);
                }
            })
            .map_err(|e| eyre!("failed to render: {}", e))?;

        res
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
        self.store.dispatch(Action::SetUIPaused(true))?;
        self.ipc.tx.send(MainMessage::UIPaused)
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
        self.store.dispatch(Action::SetUIPaused(false))?;
        self.ipc.tx.send(MainMessage::UIResumed)
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
