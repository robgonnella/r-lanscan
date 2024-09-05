use crate::ui::store::{action::Action, dispatcher::Dispatcher, types::ViewName};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{palette::tailwind, Style},
    text::Line,
    widgets::{Block, BorderType, Paragraph},
    Frame,
};
use std::sync::Arc;

use super::View;

const INFO_TEXT: &str = "(Esc) back to main view";

pub struct DeviceView {
    pub id: ViewName,
    dispatcher: Arc<Dispatcher>,
}

impl DeviceView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        Self {
            id: ViewName::Device,
            dispatcher,
        }
    }

    fn render_footer(&mut self, f: &mut Frame, area: Rect) {
        let colors = self.dispatcher.get_state().colors;
        let info_footer = Paragraph::new(Line::from(INFO_TEXT))
            .style(
                Style::new()
                    .fg(tailwind::SLATE.c200)
                    .bg(tailwind::SLATE.c950),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(colors.footer_border_color)),
            );
        f.render_widget(info_footer, area);
    }
}

impl View for DeviceView {
    fn render(&mut self, f: &mut Frame) {
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(f.area());
        self.render_footer(f, rects[1]);
    }

    fn process_key_event(&mut self, key: KeyEvent) -> bool {
        let mut handled = false;
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Esc => {
                    self.dispatcher
                        .dispatch(Action::UpdateView(&ViewName::Devices));
                    handled = true;
                }
                _ => {}
            }
        }

        handled
    }
}
