use ratatui::{
    layout::{Margin, Rect},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::ui::store::store::Colors;

use super::Component;

pub struct ScrollBar<'s> {
    scroll_state: &'s mut ScrollbarState,
}

impl<'s> ScrollBar<'s> {
    pub fn new(scroll_state: &'s mut ScrollbarState) -> Self {
        Self { scroll_state }
    }
}

impl<'s> Component for ScrollBar<'s> {
    fn render(&mut self, f: &mut Frame, area: Rect, _colors: &Colors) {
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }
}
