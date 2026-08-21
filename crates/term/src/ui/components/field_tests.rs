use super::*;
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_field_component() {
    let field = Field::new("Test".to_string(), "value".to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 1)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(field, frame.area()))
        .unwrap();
    assert_snapshot!(terminal.backend());
}
