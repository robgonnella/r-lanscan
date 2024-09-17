use ratatui::{
    layout::{Margin, Rect},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

pub struct ScrollBar {}

impl ScrollBar {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for ScrollBar {
    type State = ScrollbarState;

    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        let scroll_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        scrollbar.render(scroll_area, buf, state)
    }
}
