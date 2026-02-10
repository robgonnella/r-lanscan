use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

use crate::store::{
    Dispatcher, StateGetter, Store, action::Action, reducer::StoreReducer,
    state::State,
};

use super::*;

fn setup() -> (LogsView, Store) {
    let store = Store::new(State::default(), StoreReducer::boxed());

    store.dispatch(Action::Log("test log 1".into()));
    store.dispatch(Action::Log("test log 2".into()));

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
