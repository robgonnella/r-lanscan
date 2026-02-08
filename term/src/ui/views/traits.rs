//! Traits for UI components, views, and event handling.

use color_eyre::eyre::Result;
use ratatui::{crossterm::event::Event as CrossTermEvent, layout::Rect};

use crate::{
    ipc::{message::MainMessage, traits::IpcSender},
    ui::store::state::State,
};

/// Context passed to widgets during rendering and event handling.
pub struct CustomEventContext<'a> {
    // app state
    pub state: &'a State,
    // event producer - this how components and views can communicate user
    // behavior back to main loop to perform actions that aren't related to
    // state - executing a shell command
    pub ipc: Box<dyn IpcSender<MainMessage>>,
}

/// Handles keyboard and mouse events, returns true if consumed.
pub trait EventHandler {
    fn process_event(
        &self,
        evt: &CrossTermEvent,
        ctx: &CustomEventContext,
    ) -> Result<bool>;
}

/// Context passed to widgets during rendering and event handling.
pub struct CustomWidgetContext<'a> {
    // app state
    pub state: &'a State,
    // total area for the entire application - useful for calculating
    // popover areas
    pub app_area: Rect,
}

/// Owned widget that consumes self on render.
pub trait CustomWidget {
    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    );
}

/// Borrowed widget that can render multiple times.
pub trait CustomWidgetRef {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()>;
}

/// Stateful widget with mutable render state.
pub trait CustomStatefulWidget {
    type State;

    fn render(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        ctx: &CustomWidgetContext,
    );
}

/// A screen that handles events and renders content.
pub trait View: EventHandler + CustomWidgetRef {
    fn legend(&self, _state: &State) -> String {
        "".into()
    }
    fn override_main_legend(&self, _state: &State) -> bool {
        false
    }
}
