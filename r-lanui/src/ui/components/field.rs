use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::ui::{store::state::State, views::traits::CustomWidget};

pub struct Field {
    key: String,
    value: String,
}

impl Field {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

impl CustomWidget for Field {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, _state: &State)
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
