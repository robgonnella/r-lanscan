//! Editable text input component.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::ui::views::traits::{CustomStatefulWidget, CustomWidgetContext};

/// State for an input field (editing mode and current value).
#[derive(Debug, Clone)]
pub struct InputState {
    pub editing: bool,
    pub value: String,
}

/// Labeled text input that highlights when in edit mode.
pub struct Input {
    label: String,
}

impl Input {
    /// Creates a new input with the given label.
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
#[path = "./input_tests.rs"]
mod tests;
