//! Header component for section titles.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::ui::views::traits::{CustomWidget, CustomWidgetContext};

/// Bold styled section header.
pub struct Label {
    title: String,
    width: Option<u16>,
    padding_x: Option<u16>,
    padding_y: Option<u16>,
}

impl Label {
    /// Creates a new header with the given title.
    pub fn new(title: String) -> Self {
        Self {
            title,
            width: None,
            padding_x: None,
            padding_y: None,
        }
    }

    #[allow(dead_code)]
    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    #[allow(dead_code)]
    pub fn padding_x(mut self, x: u16) -> Self {
        self.padding_x = Some(x);
        self
    }

    #[allow(dead_code)]
    pub fn padding_y(mut self, y: u16) -> Self {
        self.padding_y = Some(y);
        self
    }
}

impl CustomWidget for Label {
    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) where
        Self: Sized,
    {
        let width = self.width.as_ref().unwrap_or(&25);
        let padding_x = self.padding_x.as_ref().unwrap_or(&0);
        let padding_y = self.padding_y.as_ref().unwrap_or(&0);

        let [label_area, _] = Layout::horizontal([
            Constraint::Length(*width),
            Constraint::Min(0),
        ])
        .areas(area);

        let block = Block::new()
            .padding(Padding::symmetric(*padding_x, *padding_y))
            .style(Style::default().bg(ctx.state.colors.row_header_bg));

        let inner_area = block.inner(label_area);

        block.render(label_area, buf);

        let style = Style::default()
            .bg(ctx.state.colors.row_header_bg)
            .fg(ctx.state.colors.text)
            .add_modifier(Modifier::BOLD);

        Paragraph::new(self.title)
            .style(style)
            .render(inner_area, buf);
    }
}

#[cfg(test)]
#[path = "./label_tests.rs"]
mod tests;
