use crate::ui::{
    components::{field::Field, header::Header},
    store::{
        action::Action,
        dispatcher::Dispatcher,
        state::{State, Theme, ViewID},
    },
};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::WidgetRef,
};
use std::sync::Arc;

use super::{CustomWidget, EventHandler, View};

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
        self.set_colors();
    }

    fn previous_color(&mut self) {
        let count = THEMES.len();
        self.theme_index = (self.theme_index + count - 1) % count;
        self.set_colors();
    }

    fn set_colors(&mut self) {
        let state = self.dispatcher.get_state();
        self.dispatcher.dispatch(Action::UpdateTheme((
            &state.config.id,
            &THEMES[self.theme_index],
        )));
    }

    fn render_label(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let header = Header::new(String::from("Config"));
        header.render(area, buf, state);
    }

    fn render_network(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let field = Field::new(String::from("Network"), state.config.cidr.clone());
        field.render(area, buf, state);
    }

    fn render_ports(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let port_list = Field::new(
            String::from("Scanning Ports"),
            state.config.ports.join(", "),
        );

        port_list.render(area, buf, state);
    }

    fn render_ssh(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let rects = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1), // spacer
            Constraint::Length(1),
            Constraint::Length(1), // spacer
            Constraint::Length(1),
        ])
        .split(area);

        let ssh_user = Field::new(
            String::from("Default SSH User"),
            state.config.default_ssh_user.clone(),
        );
        let ssh_port = Field::new(
            String::from("Default SSH Port"),
            state.config.default_ssh_port.clone(),
        );
        let ssh_ident = Field::new(
            String::from("Default SSH Identity"),
            state.config.default_ssh_identity.clone(),
        );

        ssh_user.render(rects[0], buf, state);
        ssh_port.render(rects[2], buf, state);
        ssh_ident.render(rects[4], buf, state);
    }

    fn render_theme(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let value = format!("<- {0} ->", state.config.theme);
        let field = Field::new(String::from("Theme"), value);
        field.render(area, buf, state);
    }
}

impl View for ConfigView {
    fn id(&self) -> ViewID {
        ViewID::Config
    }
}

impl WidgetRef for ConfigView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        let view_rects = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacer
            Constraint::Length(1), // network
            Constraint::Length(1), // spacer
            Constraint::Length(5), // ssh
            Constraint::Length(1), // spacer
            Constraint::Length(1), // theme
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // ports
        ])
        .split(area);

        let label_rects = Layout::horizontal([Constraint::Length(20)]).split(view_rects[0]);

        self.render_label(label_rects[0], buf, &state);
        self.render_network(view_rects[2], buf, &state);
        self.render_ssh(view_rects[4], buf, &state);
        self.render_theme(view_rects[6], buf, &state);
        self.render_ports(view_rects[8], buf, &state);
    }
}

impl EventHandler for ConfigView {
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
                        KeyCode::Right => {
                            self.next_color();
                            handled = true;
                        }
                        KeyCode::Left => {
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
