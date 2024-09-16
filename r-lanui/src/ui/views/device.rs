use crate::ui::{
    components::footer::InfoFooter,
    store::{action::Action, dispatcher::Dispatcher, store::Colors, types::ViewName},
};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    Frame,
};
use std::sync::Arc;

use super::View;
use crate::ui::components::Component;

const INFO_TEXT: &str = "(Esc) back to main view";

pub struct DeviceView {
    dispatcher: Arc<Dispatcher>,
}

impl DeviceView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        Self { dispatcher }
    }

    fn render_footer(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let mut footer = InfoFooter::new(INFO_TEXT.to_string());
        footer.render(f, area, colors);
    }
}

impl View for DeviceView {
    fn render(&mut self, f: &mut Frame) {
        let colors = self.dispatcher.get_state().colors;
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(f.area());
        self.render_footer(f, rects[1], &colors);
    }

    fn process_event(&mut self, evt: &Event) -> bool {
        let mut handled = false;
        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(_m) => {}
            Event::Paste(_s) => {}
            Event::Resize(_x, _y) => {}
            Event::Key(key) => {
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
            }
        }

        handled
    }
}
