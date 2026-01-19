use crate::ui::store::state::State;

use super::*;
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_header_component() {
    let header = Header::new("Test".to_string());
    let mut terminal = Terminal::new(TestBackend::new(80, 3)).unwrap();
    let state = State::default();
    let channel = std::sync::mpsc::channel();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
                events: channel.0,
            };

            header.render(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
