use crate::{
    ipc::{message::MainMessage, traits::MockIpcSender},
    ui::store::state::State,
};

use super::*;
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_header_component() {
    let header = Header::new("Test".to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 3)).unwrap();
    let state = State::default();
    let sender = MockIpcSender::<MainMessage>::new();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
                ipc: Box::new(sender),
            };

            header.render(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
