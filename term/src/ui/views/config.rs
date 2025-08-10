use crate::ui::{
    colors::Theme,
    components::{
        field::Field,
        header::Header,
        input::{Input, InputState},
    },
    store::{
        action::Action,
        state::{State, ViewID},
        Store,
    },
};
use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};
use std::{cell::RefCell, sync::Arc};

use super::traits::{
    CustomStatefulWidget, CustomWidget, CustomWidgetContext, CustomWidgetRef, EventHandler, View,
};

const THEMES: [Theme; 4] = [Theme::Blue, Theme::Emerald, Theme::Indigo, Theme::Red];

#[derive(Debug, Clone)]
enum Focus {
    SSHUser,
    SSHPort,
    SSHIdentity,
    Theme,
    ScanPorts,
}

pub struct ConfigView {
    store: Arc<Store>,
    theme_index: RefCell<usize>,
    editing: RefCell<bool>,
    focus: RefCell<Focus>,
    ssh_user_state: RefCell<InputState>,
    ssh_port_state: RefCell<InputState>,
    ssh_identity_state: RefCell<InputState>,
    theme_state: RefCell<InputState>,
    scan_ports_state: RefCell<InputState>,
}

impl ConfigView {
    pub fn new(store: Arc<Store>) -> Self {
        let state = store.get_state();

        let theme = Theme::from_string(&state.config.theme);

        let (idx, _) = THEMES
            .iter()
            .enumerate()
            .find(|(_, t)| **t == theme)
            .unwrap();

        Self {
            store,
            theme_index: RefCell::new(idx),
            editing: RefCell::new(false),
            focus: RefCell::new(Focus::SSHUser),
            ssh_user_state: RefCell::new(InputState {
                editing: false,
                value: String::from(""),
            }),
            ssh_port_state: RefCell::new(InputState {
                editing: false,
                value: String::from(""),
            }),
            ssh_identity_state: RefCell::new(InputState {
                editing: false,
                value: String::from(""),
            }),
            theme_state: RefCell::new(InputState {
                editing: false,
                value: String::from(""),
            }),
            scan_ports_state: RefCell::new(InputState {
                editing: false,
                value: String::from(""),
            }),
        }
    }

    fn next_color(&self) {
        let new_idx = *self.theme_index.borrow() + 1;
        *self.theme_index.borrow_mut() = new_idx % THEMES.len();
        let theme = THEMES[*self.theme_index.borrow()].clone();
        self.theme_state.borrow_mut().value = theme.to_string();
        self.store.dispatch(Action::PreviewTheme(theme));
    }

    fn previous_color(&self) {
        let count = THEMES.len();
        let new_idx = *self.theme_index.borrow() + count - 1;
        *self.theme_index.borrow_mut() = new_idx % count;
        let theme = THEMES[*self.theme_index.borrow()].clone();
        self.theme_state.borrow_mut().value = theme.to_string();
        self.store.dispatch(Action::PreviewTheme(theme));
    }

    fn set_config(&self, state: &State) {
        let mut config = state.config.clone();
        config.theme = THEMES[*self.theme_index.borrow()].clone().to_string();
        config.default_ssh_user = self.ssh_user_state.borrow().value.clone();
        let mut port = self.ssh_port_state.borrow().value.clone().parse::<u16>();
        if port.is_err() {
            port = Ok(22)
        }
        config.default_ssh_port = port.unwrap().to_string();
        config.default_ssh_identity = self.ssh_identity_state.borrow().value.clone();
        config.ports = self
            .scan_ports_state
            .borrow()
            .value
            .clone()
            .split(",")
            .map_into()
            .collect();
        self.store.dispatch(Action::UpdateConfig(config));
    }

    fn render_label(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let header = Header::new("Config".to_string());
        header.render(area, buf, ctx);
    }

    fn render_network(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let field = Field::new("Network".to_string(), ctx.state.config.cidr.clone());
        field.render(area, buf);
    }

    fn push_input_char(&self, char: char) {
        match *self.focus.borrow() {
            Focus::SSHUser => self.ssh_user_state.borrow_mut().value.push(char),
            Focus::SSHPort => self.ssh_port_state.borrow_mut().value.push(char),
            Focus::SSHIdentity => self.ssh_identity_state.borrow_mut().value.push(char),
            Focus::ScanPorts => self.scan_ports_state.borrow_mut().value.push(char),
            _ => {}
        };
    }

    fn pop_input_char(&self) {
        match *self.focus.borrow() {
            Focus::SSHUser => {
                self.ssh_user_state.borrow_mut().value.pop();
            }
            Focus::SSHPort => {
                self.ssh_port_state.borrow_mut().value.pop();
            }
            Focus::SSHIdentity => {
                self.ssh_identity_state.borrow_mut().value.pop();
            }
            Focus::ScanPorts => {
                self.scan_ports_state.borrow_mut().value.pop();
            }
            _ => {}
        };
    }

    fn reset_input_state(&self) {
        self.ssh_user_state.borrow_mut().editing = false;
        self.ssh_port_state.borrow_mut().editing = false;
        self.ssh_identity_state.borrow_mut().editing = false;
        self.theme_state.borrow_mut().editing = false;
        self.scan_ports_state.borrow_mut().editing = false;
    }

    fn focus_next(&self) {
        let next_focus = match *self.focus.borrow() {
            Focus::SSHUser => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::SSHPort
            }
            Focus::SSHPort => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
            Focus::SSHIdentity => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = true;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::Theme
            }
            Focus::Theme => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = true;
                }
                Focus::ScanPorts
            }
            Focus::ScanPorts => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::SSHUser
            }
        };

        *self.focus.borrow_mut() = next_focus;
    }

    fn focus_previous(&self) {
        let next_focus = match *self.focus.borrow() {
            Focus::ScanPorts => {
                if *self.editing.borrow() {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::Theme
            }
            Focus::Theme => {
                if *self.editing.borrow() {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
            Focus::SSHIdentity => {
                if *self.editing.borrow() {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHPort
            }
            Focus::SSHPort => {
                if *self.editing.borrow() {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = true;
                }
                Focus::SSHUser
            }
            Focus::SSHUser => {
                if *self.editing.borrow() {
                    self.scan_ports_state.borrow_mut().editing = true;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
        };

        *self.focus.borrow_mut() = next_focus;
    }

    fn render_ports(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let port_list = Input::new("Scanning Ports");
        port_list.render(area, buf, &mut self.scan_ports_state.borrow_mut(), ctx);
    }

    fn render_ssh(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
        ctx: &CustomWidgetContext,
    ) {
        let rects = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1), // spacer
            Constraint::Length(1),
            Constraint::Length(1), // spacer
            Constraint::Length(1),
        ])
        .split(area);

        if !*self.editing.borrow() {
            self.ssh_user_state.borrow_mut().value = state.config.default_ssh_user.clone();
            self.ssh_port_state.borrow_mut().value = state.config.default_ssh_port.clone();
            self.ssh_identity_state.borrow_mut().value = state.config.default_ssh_identity.clone();
            self.theme_state.borrow_mut().value = state.config.theme.clone();
            self.scan_ports_state.borrow_mut().value = state.config.ports.join(",");
        }

        let ssh_user = Input::new("Default SSH User");
        let ssh_port = Input::new("Default SSH Port");
        let ssh_ident = Input::new("Default SSH Identity");

        ssh_user.render(rects[0], buf, &mut self.ssh_user_state.borrow_mut(), ctx);
        ssh_port.render(rects[2], buf, &mut self.ssh_port_state.borrow_mut(), ctx);
        ssh_ident.render(
            rects[4],
            buf,
            &mut self.ssh_identity_state.borrow_mut(),
            ctx,
        );
    }

    fn render_theme(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let input = Input::new("Theme <->");
        input.render(area, buf, &mut self.theme_state.borrow_mut(), ctx);
    }
}

impl View for ConfigView {
    fn id(&self) -> ViewID {
        ViewID::Config
    }
    fn legend(&self, _state: &State) -> &str {
        if *self.editing.borrow() {
            "(esc) exit configuration | (tab) focus next | (enter) save config"
        } else {
            "(c) configure"
        }
    }
    fn override_main_legend(&self, _state: &State) -> bool {
        *self.editing.borrow()
    }
}

impl CustomWidgetRef for ConfigView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
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

        self.render_label(label_rects[0], buf, ctx);
        self.render_network(view_rects[2], buf, ctx);
        self.render_ssh(view_rects[4], buf, &ctx.state, ctx);
        self.render_theme(view_rects[6], buf, ctx);
        self.render_ports(view_rects[8], buf, ctx);
    }
}

impl EventHandler for ConfigView {
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if ctx.state.render_view_select {
            return false;
        }

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
                            if *self.editing.borrow() {
                                self.reset_input_state();
                                *self.focus.borrow_mut() = Focus::SSHUser;
                                *self.editing.borrow_mut() = false;
                                handled = true;
                            }
                        }
                        KeyCode::Tab => {
                            if *self.editing.borrow() {
                                self.focus_next();
                                handled = true;
                            }
                        }
                        KeyCode::BackTab => {
                            if *self.editing.borrow() {
                                self.focus_previous();
                                handled = true;
                            }
                        }
                        KeyCode::Right => {
                            if *self.editing.borrow() && self.theme_state.borrow().editing {
                                self.next_color();
                                handled = true;
                            }
                        }
                        KeyCode::Left => {
                            if *self.editing.borrow() && self.theme_state.borrow().editing {
                                self.previous_color();
                                handled = true;
                            }
                        }
                        KeyCode::Enter => {
                            if *self.editing.borrow() {
                                self.set_config(&ctx.state);
                                self.reset_input_state();
                                *self.focus.borrow_mut() = Focus::SSHUser;
                                *self.editing.borrow_mut() = false;
                                handled = true;
                            }
                        }
                        KeyCode::Backspace => {
                            if *self.editing.borrow() && !self.theme_state.borrow().editing {
                                self.pop_input_char();
                                handled = true;
                            }
                        }
                        KeyCode::Char(c) => {
                            if *self.editing.borrow() && !self.theme_state.borrow().editing {
                                // handle value update for focused element
                                self.push_input_char(c);
                                handled = true;
                            } else if c == 'c' && !*self.editing.borrow() {
                                // enter edit mode
                                *self.editing.borrow_mut() = true;
                                self.ssh_user_state.borrow_mut().editing = true;
                                handled = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        handled
    }
}

#[cfg(test)]
#[path = "./config_tests.rs"]
mod tests;
