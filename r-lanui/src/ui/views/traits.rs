use ratatui::{crossterm::event::Event, layout::Rect};

use crate::ui::store::state::{State, ViewID};

pub trait EventHandler {
    fn process_event(&self, evt: &Event, state: &State) -> bool;
}

pub trait CustomWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State);
}

pub trait CustomWidgetRef {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
        // gives views access to total app area for things like
        // calculating popover area from total available area
        total_area: Rect,
    );
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

pub trait View: EventHandler + CustomWidgetRef {
    fn id(&self) -> ViewID;
    fn legend(&self, _state: &State) -> &str {
        ""
    }
    fn override_main_legend(&self, _state: &State) -> bool {
        false
    }
}
