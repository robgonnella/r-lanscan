use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::ui::{store::state::State, views::CustomWidget};

pub struct Header {
    title: String,
}

impl Header {
    pub fn new(title: String) -> Self {
        Self { title }
    }
}

impl CustomWidget for Header {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State)
    where
        Self: Sized,
    {
        let header_style = Style::default()
            .fg(state.colors.border_focused_color)
            .add_modifier(Modifier::BOLD);

        let header = Paragraph::new(Line::from(self.title.as_str())).style(header_style);

        header.render(area, buf)
    }
}
