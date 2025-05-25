use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::ui::views::traits::{CustomStatefulWidget, CustomWidgetContext};

#[derive(Debug, Clone)]
pub struct InputState {
    pub editing: bool,
    pub value: String,
}

pub struct Input {
    label: String,
}

impl Input {
    pub fn new(label: &str) -> Self {
        Self {
            label: String::from(label),
        }
    }
}

impl CustomStatefulWidget for Input {
    type State = InputState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
        ctx: &CustomWidgetContext,
    ) where
        Self: Sized,
    {
        let label = Span::from(format!("{0}: ", self.label));
        let mut style = Style::default();
        if state.editing {
            style = style.fg(ctx.state.colors.input_editing);
        }
        let value = Span::from(state.value.as_str()).style(style);
        let line = Line::from(vec![label, value]);
        line.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::store::state::State;

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_input_component_non_edit_mode() {
        let input = Input::new("Test");

        let mut input_state = InputState {
            editing: false,
            value: "value".to_string(),
        };

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

                input.render(frame.area(), frame.buffer_mut(), &mut input_state, &ctx);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn renders_input_component_edit_mode() {
        let input = Input::new("Test");

        let mut input_state = InputState {
            editing: true,
            value: "value".to_string(),
        };

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

                input.render(frame.area(), frame.buffer_mut(), &mut input_state, &ctx);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
