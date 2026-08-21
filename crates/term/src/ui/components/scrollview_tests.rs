use crate::{store::state::State, ui::components::table::DEFAULT_ITEM_HEIGHT};

use super::*;
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_scrollview_component() {
    let line = "a long line of text that we want to wrap and scroll, it's got lots of useful info in a small area and loves run-on sentences like this one, which is totally awesome, don't you think, I think so";
    let mut text = format!("1: {line}");
    for i in 1..99 {
        text = format!("{text}\n\n{}: {line}", i + 1);
    }
    let view = ScrollView::new(&text);
    let state = State::default();

    let mut terminal = Terminal::new(TestBackend::new(100, 100)).unwrap();

    terminal
        .draw(|frame| {
            let frame_area = frame.area();
            let height = frame_area.height as usize;

            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame_area,
            };

            let mut scroll_state = ScrollbarState::new(DEFAULT_ITEM_HEIGHT)
                .content_length(100)
                .viewport_content_length(height)
                .position(50);

            view.render(
                frame.area(),
                frame.buffer_mut(),
                &mut scroll_state,
                &ctx,
            );
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
