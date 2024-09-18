use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::ui::store::store::Colors;

pub struct InfoFooter<'c> {
    content: String,
    colors: &'c Colors,
}

impl<'c> InfoFooter<'c> {
    pub fn new(content: String, colors: &'c Colors) -> Self {
        Self { content, colors }
    }
}

impl<'c> Widget for InfoFooter<'c> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let info_footer = Paragraph::new(Line::from(self.content.as_str()))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );

        info_footer.render(area, buf)
    }
}
