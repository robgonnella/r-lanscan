//! Header component for section titles.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::ui::views::traits::{CustomWidget, CustomWidgetContext};

/// Bold styled section header.
pub struct Header {
    title: String,
}

impl Header {
    /// Creates a new header with the given title.
    pub fn new(title: String) -> Self {
        Self { title }
    }
}

impl CustomWidget for Header {
    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) where
        Self: Sized,
    {
        let header_style = Style::default()
            .fg(ctx.state.colors.label)
            .add_modifier(Modifier::BOLD);

        let header =
            Paragraph::new(Line::from(self.title.as_str())).style(header_style);

        header.render(area, buf)
    }
}

#[cfg(test)]
#[path = "./header_tests.rs"]
mod tests;
