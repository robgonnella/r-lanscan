use std::{cell::RefCell, rc::Rc};

use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    store::state::State,
    ui::{
        components::{
            input::InputState,
            popover::{base::Popover, browse::BrowsePopover},
        },
        views::traits::{CustomWidgetContext, CustomWidgetRef},
    },
};

#[test]
fn renders_browse_popover_component() {
    let browser_state = Rc::new(RefCell::new(InputState {
        editing: false,
        value: "lynx".into(),
    }));

    let port_state = Rc::new(RefCell::new(InputState {
        editing: true,
        value: "80".into(),
    }));

    let browse = BrowsePopover::new(browser_state, port_state);

    let popover = Popover::new(&browse);

    let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
    let state = State::default();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };

            popover
                .render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
