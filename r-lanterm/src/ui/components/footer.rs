use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::ui::views::traits::{CustomWidget, CustomWidgetContext};

pub struct InfoFooter {
    content: String,
}

impl InfoFooter {
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl CustomWidget for InfoFooter {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, ctx: &CustomWidgetContext)
    where
        Self: Sized,
    {
        let info_footer = Paragraph::new(Line::from(self.content.as_str()))
            .style(
                Style::new()
                    .fg(ctx.state.colors.row_fg)
                    .bg(ctx.state.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(ctx.state.colors.border_color)),
            );

        info_footer.render(area, buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::store::state::State;

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_footer_component() {
        let footer = InfoFooter::new("Test".to_string());
        let mut terminal = Terminal::new(TestBackend::new(80, 3)).unwrap();
        let state = State::default();
        let channel = std::sync::mpsc::channel();

        terminal
            .draw(|frame| {
                let ctx = CustomWidgetContext {
                    state,
                    app_area: frame.area(),
                    events: channel.0,
                };

                footer.render(frame.area(), frame.buffer_mut(), &ctx);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
