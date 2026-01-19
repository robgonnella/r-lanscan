use crate::ui::store::state::State;

use super::*;
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_input_component_non_edit_mode() {
    let input = Input::new("Test");

    let mut input_state = InputState {
        editing: false,
        value: "value".to_string(),
    };

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

            input.render(frame.area(), frame.buffer_mut(), &mut input_state, &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn renders_input_component_edit_mode() {
    let input = Input::new("Test");

    let mut input_state = InputState {
        editing: true,
        value: "value".to_string(),
    };

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

            input.render(frame.area(), frame.buffer_mut(), &mut input_state, &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
