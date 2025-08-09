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
