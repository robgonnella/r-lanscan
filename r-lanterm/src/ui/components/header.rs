use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::ui::views::traits::{CustomWidget, CustomWidgetContext};

pub struct Header {
    title: String,
}

impl Header {
    pub fn new(title: String) -> Self {
        Self { title }
    }
}

impl CustomWidget for Header {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, ctx: &CustomWidgetContext)
    where
        Self: Sized,
    {
        let header_style = Style::default()
            .fg(ctx.state.colors.label)
            .add_modifier(Modifier::BOLD);

        let header = Paragraph::new(Line::from(self.title.as_str())).style(header_style);

        header.render(area, buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::store::state::State;

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_header_component() {
        let header = Header::new("Test".to_string());
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

                header.render(frame.area(), frame.buffer_mut(), &ctx);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
