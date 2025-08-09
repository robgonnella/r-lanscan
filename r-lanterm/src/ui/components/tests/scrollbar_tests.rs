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
