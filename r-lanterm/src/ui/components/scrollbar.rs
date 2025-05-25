use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use crate::ui::views::traits::{CustomStatefulWidget, CustomWidgetContext};

pub struct ScrollBar {}

impl ScrollBar {
    pub fn new() -> Self {
        Self {}
    }
}

impl CustomStatefulWidget for ScrollBar {
    type State = ScrollbarState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        ctx: &CustomWidgetContext,
    ) {
        let scroll_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        if scroll_area.width < 1 || scroll_area.height < 1 {
            return;
        }

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .style(Style::new().fg(ctx.state.colors.scroll_bar_fg));

        scrollbar.render(scroll_area, buf, state)
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::store::state::State;

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_scrollbar_component() {
        let scroll = ScrollBar::new();
        let mut scroll_state = ScrollbarState::new(10);
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

                scroll.render(frame.area(), frame.buffer_mut(), &mut scroll_state, &ctx);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
