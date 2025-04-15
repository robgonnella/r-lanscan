use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::ui::{store::state::State, views::traits::CustomWidget};

pub struct InfoFooter {
    content: String,
}

impl InfoFooter {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl CustomWidget for InfoFooter {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State)
    where
        Self: Sized,
    {
        let info_footer = Paragraph::new(Line::from(self.content.as_str()))
            .style(
                Style::new()
                    .fg(state.colors.row_fg)
                    .bg(state.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(state.colors.border_color)),
            );

        info_footer.render(area, buf)
    }
}
