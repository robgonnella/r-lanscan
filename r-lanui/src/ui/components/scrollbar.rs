use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use crate::ui::{store::state::State, views::CustomStatefulWidget};

pub struct ScrollBar {}

impl ScrollBar {
    pub fn new() -> Self {
        Self {}
    }
}

impl CustomStatefulWidget for ScrollBar {
    type State = ScrollbarState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        custom_state: &State,
    ) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .style(Style::new().fg(custom_state.colors.scroll_bar_fg));

        let scroll_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        scrollbar.render(scroll_area, buf, state)
    }
}
