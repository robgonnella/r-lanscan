//! Traits for UI components, views, and event handling.

use ratatui::{crossterm::event::Event as CrossTermEvent, layout::Rect};

use crate::{
    ipc::{message::MainMessage, traits::IpcSender},
    ui::store::state::{State, ViewID},
};

/// Handles keyboard and mouse events, returns true if consumed.
pub trait EventHandler {
    fn process_event(&self, evt: &CrossTermEvent, ctx: &CustomWidgetContext) -> bool;
}

/// Context passed to widgets during rendering and event handling.
pub struct CustomWidgetContext<'a> {
    // app state
    pub state: &'a State,
    // total area for the entire application - useful for calculating
    // popover areas
    pub app_area: Rect,
    // event producer - this how components and views can communicate user
    // behavior back to main loop to perform actions that aren't related to
    // state - executing a shell command
    pub ipc: Box<dyn IpcSender<MainMessage>>,
}

/// Owned widget that consumes self on render.
pub trait CustomWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, ctx: &CustomWidgetContext);
}

/// Borrowed widget that can render multiple times.
pub trait CustomWidgetRef {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, ctx: &CustomWidgetContext);
}

/// Stateful widget with mutable render state.
pub trait CustomStatefulWidget {
    type State;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        ctx: &CustomWidgetContext,
    );
}

/// A screen that handles events and renders content.
pub trait View: EventHandler + CustomWidgetRef {
    fn id(&self) -> ViewID;
    fn legend(&self, _state: &State) -> &str {
        ""
    }
    fn override_main_legend(&self, _state: &State) -> bool {
        false
    }
}
