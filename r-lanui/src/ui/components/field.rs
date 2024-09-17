use ratatui::{layout::Rect, style::Style, text::Line, widgets::Paragraph};

use crate::ui::store::store::Colors;

use super::Component;

pub struct Field {
    pub name: String,
    pub value: String,
}

impl Field {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

impl Component for Field {
    fn render(&mut self, f: &mut ratatui::Frame, area: Rect, colors: &Colors) {
        let field = Paragraph::new(Line::from(format!("{0}: {1}", self.name, self.value)))
            .style(Style::new().fg(colors.row_fg).bg(colors.buffer_bg))
            .left_aligned();

        f.render_widget(field, area);
    }
}
