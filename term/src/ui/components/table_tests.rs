use crate::ui::store::state::State;

use super::*;
use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn renders_table_component() {
    let items = vec![vec!["Test".to_string()]];
    let headers = Some(vec!["Items".to_string()]);
    let col_sizes = vec![10];
    let table = Table::new(items, headers, col_sizes, 2);
    let state = State::default();
    let channel = std::sync::mpsc::channel();
    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state,
                app_area: frame.area(),
                events: channel.0,
            };

            table.render_ref(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
