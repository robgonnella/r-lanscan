use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

use crate::store::{
    Dispatcher, StateGetter, Store, action::Action, reducer::StoreReducer,
    state::State,
};

use super::*;

fn setup() -> (LogsView, Store) {
    let mut reducer = StoreReducer::boxed();
    reducer.enable_logging();
    let store = Store::new(State::default(), reducer);

    store.dispatch(Action::UpdateMessage(Some("test message 1".into())));
    store.dispatch(Action::UpdateMessage(Some("test message 2".into())));

    (LogsView::new(), store)
}

#[test]
fn test_logs_view() {
    let (logs_view, store) = setup();
    let mut terminal = Terminal::new(TestBackend::new(130, 15)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };

            logs_view
                .render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn scroll_to_top_resets_offset_and_unpins() {
    let view = LogsView::new();
    // Simulate being somewhere in the middle.
    view.scroll_offset.replace(42);
    view.at_bottom.replace(false);

    view.scroll_to_top();

    assert_eq!(*view.scroll_offset.borrow(), 0);
    assert!(!*view.at_bottom.borrow());
}

#[test]
fn scroll_to_bottom_pins_to_bottom() {
    let view = LogsView::new();
    view.scroll_offset.replace(0);
    view.at_bottom.replace(false);

    view.scroll_to_bottom();

    assert!(*view.at_bottom.borrow());
}
