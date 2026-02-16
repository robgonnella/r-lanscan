use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    store::state::State,
    ui::{
        components::popover::{base::Popover, simple::SimplePopover},
        views::traits::{CustomWidgetContext, CustomWidgetRef},
    },
};

#[test]
fn renders_simple_popover_component() {
    let test_widget = SimplePopover::new("Test simple popover")
        .footer("test footer")
        .footer_centered();

    let popover = Popover::new(&test_widget);

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
