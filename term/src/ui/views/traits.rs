use std::sync::mpsc::Sender;

use ratatui::{crossterm::event::Event as CrossTermEvent, layout::Rect};

use crate::{
    events::types::Event,
    ui::store::state::{State, ViewID},
};

pub trait EventHandler {
    fn process_event(&self, evt: &CrossTermEvent, ctx: &CustomWidgetContext) -> bool;
}

pub struct CustomWidgetContext<'a> {
    // app state
    pub state: &'a State,
    // total area for the entire application - useful for calculating
    // popover areas
    pub app_area: Rect,
    // event producer - this how components and views can communicate user
    // behavior back to main loop to perform actions that aren't related to
    // state - executing a shell command
    pub events: Sender<Event>,
}

pub trait CustomWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, ctx: &CustomWidgetContext);
}

pub trait CustomWidgetRef {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, ctx: &CustomWidgetContext);
}

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

pub trait View: EventHandler + CustomWidgetRef {
    fn id(&self) -> ViewID;
    fn legend(&self, _state: &State) -> &str {
        ""
    }
    fn override_main_legend(&self, _state: &State) -> bool {
        false
    }
}
