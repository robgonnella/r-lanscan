use color_eyre::eyre::Result;
use insta::assert_snapshot;
use ratatui::{
    Terminal,
    backend::TestBackend,
    widgets::{Paragraph, Widget},
};

use crate::{
    store::state::State,
    ui::{
        components::popover::base::Popover,
        views::traits::{CustomWidgetContext, CustomWidgetRef},
    },
};

#[derive(Default)]
struct TestWidget;

impl CustomWidgetRef for TestWidget {
    fn render_ref(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        _ctx: &CustomWidgetContext,
    ) -> Result<()> {
        Paragraph::new("Test").render(area, buf);
        Ok(())
    }
}

#[test]
fn renders_base_popover_component() {
    let test_widget = TestWidget;
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
