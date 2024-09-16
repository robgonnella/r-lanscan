use crate::ui::{
    components::{footer::InfoFooter, Component},
    store::{
        action::Action,
        dispatcher::Dispatcher,
        store::Colors,
        types::{Theme, ViewName},
    },
};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    Frame,
};
use std::sync::Arc;

use super::View;

const INFO_TEXT: &str =
    "(Esc) back to main view | | (→) next color | (←) previous color | (Enter) Save";

const THEMES: [Theme; 4] = [Theme::Blue, Theme::Emerald, Theme::Indigo, Theme::Red];

pub struct ConfigView {
    dispatcher: Arc<Dispatcher>,
    theme_index: usize,
}

impl ConfigView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let state = dispatcher.get_state();
        let theme = Theme::from_string(&state.config.theme);
        let (idx, _) = THEMES
            .iter()
            .enumerate()
            .find(|(_, t)| **t == theme)
            .unwrap();
        Self {
            dispatcher,
            theme_index: idx,
        }
    }

    fn next_color(&mut self) {
        self.theme_index = (self.theme_index + 1) % THEMES.len();
    }

    fn previous_color(&mut self) {
        let count = THEMES.len();
        self.theme_index = (self.theme_index + count - 1) % count;
    }

    fn set_colors(&mut self) {
        let state = self.dispatcher.get_state();
        self.dispatcher.dispatch(Action::UpdateTheme((
            &state.config.id,
            &THEMES[self.theme_index],
        )));
    }

    fn get_colors(&self) -> Colors {
        let theme = THEMES[self.theme_index].clone();
        Colors::new(theme.to_palette())
    }

    fn _render_ssh_overrides(&mut self, _f: &mut Frame, _area: Rect) {}

    fn _render_ports(&mut self, _f: &mut Frame, _area: Rect) {}

    fn _render_save(&mut self, _f: &mut Frame, _area: Rect) {}

    fn _render_reset(&mut self, _f: &mut Frame, _area: Rect) {}

    fn render_footer(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let mut footer = InfoFooter::new(INFO_TEXT.to_string());
        footer.render(f, area, colors);
    }
}

impl View for ConfigView {
    fn render(&mut self, f: &mut Frame) {
        let colors: Colors = self.get_colors();
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
                        KeyCode::Char('l') | KeyCode::Right => {
                            self.next_color();
                            handled = true;
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            self.previous_color();
                            handled = true;
                        }
                        KeyCode::Enter => {
                            self.set_colors();
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
