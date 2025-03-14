use ratatui::{crossterm::event::Event, layout::Rect, widgets::WidgetRef};

use super::store::state::{State, ViewID};

pub mod config;
pub mod device;
pub mod devices;
pub mod main;
pub mod view_select;

pub trait EventHandler {
    fn process_event(&mut self, evt: &Event, state: &State) -> bool;
}

pub trait CustomWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State);
}

pub trait CustomWidgetRef {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State);
}

pub trait CustomStatefulWidget {
    type State;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        custom_state: &State,
    );
}

pub trait View: EventHandler + WidgetRef {
    fn id(&self) -> ViewID;
    fn legend(&self) -> &str {
        ""
    }
}
