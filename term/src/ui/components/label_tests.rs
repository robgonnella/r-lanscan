use crate::store::state::State;

use super::*;
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_label_component() {
    let header = Label::new("Test".to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 3)).unwrap();
    let state = State::default();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };

            header.render(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
