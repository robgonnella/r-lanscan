use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::ui::store::store::Colors;

pub struct Header<'c> {
    title: String,
    colors: &'c Colors,
}

impl<'c> Header<'c> {
    pub fn new(title: String, colors: &'c Colors) -> Self {
        Self { title, colors }
    }
}

impl<'c> Widget for Header<'c> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg)
            .add_modifier(Modifier::BOLD);

        let header = Paragraph::new(Line::from(self.title.as_str())).style(header_style);

        header.render(area, buf)
    }
}
