//! Scrollable table component with selection support.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Paragraph, ScrollbarState, Widget, Wrap},
};

use crate::ui::views::traits::{CustomStatefulWidget, CustomWidgetContext};

use super::scrollbar::ScrollBar;

/// Scrollable text view
pub struct ScrollView {
    text: String,
}

impl ScrollView {
    /// Creates a new scroll-view using given lines of text
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl CustomStatefulWidget for ScrollView {
    type State = ScrollbarState;

    fn render(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        ctx: &CustomWidgetContext,
    ) {
        // main view + right aligned scrollbar
        let [logs_area, scroll_bar_area] =
            Layout::horizontal([Constraint::Min(5), Constraint::Length(3)])
                .areas(area);

        let position = state.get_position() as u16;

        let p = Paragraph::new(self.text.clone())
            .wrap(Wrap { trim: true })
            .scroll((position, 0));

        p.render(logs_area, buf);

        let scrollbar = ScrollBar::new();
        scrollbar.render(scroll_bar_area, buf, state, ctx);
    }
}

// #[cfg(test)]
// #[path = "./table_tests.rs"]
// mod tests;
