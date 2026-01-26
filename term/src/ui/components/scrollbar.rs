//! Scrollbar component for tables and lists.

use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use crate::ui::views::traits::{CustomStatefulWidget, CustomWidgetContext};

/// Vertical scrollbar positioned on the right side.
pub struct ScrollBar {}

impl ScrollBar {
    /// Creates a new scrollbar.
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
        ctx: &CustomWidgetContext,
    ) {
        let scroll_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        if scroll_area.width < 1 || scroll_area.height < 1 {
            return;
        }

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .style(Style::new().fg(ctx.state.colors.scroll_bar_fg));

        scrollbar.render(scroll_area, buf, state)
    }
}

#[cfg(test)]
#[path = "./scrollbar_tests.rs"]
mod tests;
