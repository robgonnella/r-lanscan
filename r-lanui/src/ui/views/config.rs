use crate::ui::store::{
    action::Action,
    dispatcher::Dispatcher,
    types::{Theme, ViewName},
};
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

const INFO_TEXT: &str =
    "(Esc) back to main view | | (→) next color | (←) previous color | (Enter) Save";

const THEMES: [Theme; 4] = [Theme::Blue, Theme::Emerald, Theme::Indigo, Theme::Red];

pub struct ConfigView {
    pub id: ViewName,
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
            id: ViewName::Config,
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
            state.config.id,
            THEMES[self.theme_index].clone(),
        )));
    }

    fn render_footer(&mut self, f: &mut Frame, area: Rect) {
        let theme = THEMES[self.theme_index].clone();
        let palette = theme.to_palette();
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
                    .border_style(Style::new().fg(palette.c400)),
            );
        f.render_widget(info_footer, area);
    }
}

impl View for ConfigView {
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
                        .dispatch(Action::UpdateView(ViewName::Devices));
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

        handled
    }
}
