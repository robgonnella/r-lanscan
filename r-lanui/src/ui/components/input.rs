use ratatui::{
    layout::Rect,
    style::{palette::tailwind, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

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

impl StatefulWidget for Input {
    type State = InputState;

    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let label = Span::from(format!("{0}: ", self.label));
        let mut style = Style::default();
        if state.editing {
            style = style.fg(tailwind::AMBER.c600);
        }
        let value = Span::from(state.value.as_str()).style(style);
        let line = Line::from(vec![label, value]);
        line.render(area, buf);
    }
}
