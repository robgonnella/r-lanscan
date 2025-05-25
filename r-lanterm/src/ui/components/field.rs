use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

pub struct Field {
    key: String,
    value: String,
}

impl Field {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

impl Widget for Field {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let key = Span::from(format!("{0}: ", self.key));
        let value = Span::from(self.value);
        let line = Line::from(vec![key, value]);
        let field = Paragraph::new(line).wrap(Wrap { trim: true });
        field.render(area, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_field_component() {
        let field = Field::new("Test".to_string(), "value".to_string());
        let mut terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(field, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
