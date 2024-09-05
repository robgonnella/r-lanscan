use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Paragraph},
    Frame,
};

use crate::ui::store::store::Colors;

use super::Component;

pub struct InfoFooter {
    content: String,
}

impl InfoFooter {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl Component for InfoFooter {
    fn render(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let info_footer = Paragraph::new(Line::from(self.content.as_str()))
            .style(Style::new().fg(colors.row_fg).bg(colors.buffer_bg))
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(colors.footer_border_color)),
            );
        f.render_widget(info_footer, area);
    }
}
