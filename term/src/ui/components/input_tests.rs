use crate::{
    ipc::{message::MainMessage, traits::MockIpcSender},
    ui::store::state::State,
};

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
    let sender = MockIpcSender::<MainMessage>::new();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
                ipc: Box::new(sender),
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
    let sender = MockIpcSender::<MainMessage>::new();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
                ipc: Box::new(sender),
            };

            input.render(frame.area(), frame.buffer_mut(), &mut input_state, &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
