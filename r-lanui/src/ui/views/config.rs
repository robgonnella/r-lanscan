use crate::ui::{
    components::{
        field::Field,
        header::Header,
        input::{Input, InputState},
    },
    store::{
        action::Action,
        dispatcher::Dispatcher,
        state::{State, Theme, ViewID},
    },
};
use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::{StatefulWidget, WidgetRef},
};
use std::{cell::RefCell, sync::Arc};

use super::{CustomWidget, EventHandler, View};

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
    dispatcher: Arc<Dispatcher>,
    theme_index: usize,
    editing: bool,
    focus: Focus,
    ssh_user_state: RefCell<InputState>,
    ssh_port_state: RefCell<InputState>,
    ssh_identity_state: RefCell<InputState>,
    theme_state: RefCell<InputState>,
    scan_ports_state: RefCell<InputState>,
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
            editing: false,
            focus: Focus::SSHUser,
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

    fn next_color(&mut self) {
        self.theme_index = (self.theme_index + 1) % THEMES.len();
        let theme = THEMES[self.theme_index].clone();
        self.theme_state.borrow_mut().value = theme.to_string();
        self.dispatcher.dispatch(Action::PreviewTheme(theme));
    }

    fn previous_color(&mut self) {
        let count = THEMES.len();
        self.theme_index = (self.theme_index + count - 1) % count;
        let theme = THEMES[self.theme_index].clone();
        self.theme_state.borrow_mut().value = theme.to_string();
        self.dispatcher.dispatch(Action::PreviewTheme(theme));
    }

    fn set_config(&mut self, state: &State) {
        let mut config = state.config.clone();
        config.theme = THEMES[self.theme_index].clone().to_string();
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
        self.dispatcher.dispatch(Action::UpdateConfig(config));
    }

    fn render_label(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let header = Header::new(String::from("Config"));
        header.render(area, buf, state);
    }

    fn render_network(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let field = Field::new(String::from("Network"), state.config.cidr.clone());
        field.render(area, buf, state);
    }

    fn push_input_char(&self, char: char) {
        match self.focus {
            Focus::SSHUser => self.ssh_user_state.borrow_mut().value.push(char),
            Focus::SSHPort => self.ssh_port_state.borrow_mut().value.push(char),
            Focus::SSHIdentity => self.ssh_identity_state.borrow_mut().value.push(char),
            Focus::ScanPorts => self.scan_ports_state.borrow_mut().value.push(char),
            _ => {}
        };
    }

    fn pop_input_char(&self) {
        match self.focus {
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

    fn focus_next(&mut self) {
        let next_focus = match self.focus {
            Focus::SSHUser => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::SSHPort
            }
            Focus::SSHPort => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
            Focus::SSHIdentity => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = true;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::Theme
            }
            Focus::Theme => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = true;
                }
                Focus::ScanPorts
            }
            Focus::ScanPorts => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.scan_ports_state.borrow_mut().editing = false;
                }
                Focus::SSHUser
            }
        };

        self.focus = next_focus;
    }

    fn focus_previous(&mut self) {
        let next_focus = match self.focus {
            Focus::ScanPorts => {
                if self.editing {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::Theme
            }
            Focus::Theme => {
                if self.editing {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
            Focus::SSHIdentity => {
                if self.editing {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHPort
            }
            Focus::SSHPort => {
                if self.editing {
                    self.scan_ports_state.borrow_mut().editing = false;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = true;
                }
                Focus::SSHUser
            }
            Focus::SSHUser => {
                if self.editing {
                    self.scan_ports_state.borrow_mut().editing = true;
                    self.theme_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
        };

        self.focus = next_focus;
    }

    fn render_ports(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let port_list = Input::new("Scanning Ports");
        port_list.render(area, buf, &mut self.scan_ports_state.borrow_mut());
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

        if !self.editing {
            self.ssh_user_state.borrow_mut().value = state.config.default_ssh_user.clone();
            self.ssh_port_state.borrow_mut().value = state.config.default_ssh_port.clone();
            self.ssh_identity_state.borrow_mut().value = state.config.default_ssh_identity.clone();
            self.theme_state.borrow_mut().value = state.config.theme.clone();
            self.scan_ports_state.borrow_mut().value = state.config.ports.join(",");
        }

        let ssh_user = Input::new("Default SSH User");
        let ssh_port = Input::new("Default SSH Port");
        let ssh_ident = Input::new("Default SSH Identity");

        ssh_user.render(rects[0], buf, &mut self.ssh_user_state.borrow_mut());
        ssh_port.render(rects[2], buf, &mut self.ssh_port_state.borrow_mut());
        ssh_ident.render(rects[4], buf, &mut self.ssh_identity_state.borrow_mut());
    }

    fn render_theme(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let input = Input::new("Theme <->");
        input.render(area, buf, &mut self.theme_state.borrow_mut());
    }
}

impl View for ConfigView {
    fn id(&self) -> ViewID {
        ViewID::Config
    }
    fn legend(&self) -> &str {
        if self.editing {
            "(esc) exit configuration | (tab) focus next | (enter) save config"
        } else {
            "(c) configure"
        }
    }
    fn override_main_legend(&self) -> bool {
        if self.editing {
            true
        } else {
            false
        }
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
        self.render_theme(view_rects[6], buf);
        self.render_ports(view_rects[8], buf);
    }
}

impl EventHandler for ConfigView {
    fn process_event(&mut self, evt: &Event, state: &State) -> bool {
        if state.render_view_select {
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
                            if self.editing {
                                self.reset_input_state();
                                self.focus = Focus::SSHUser;
                                self.editing = false;
                                handled = true;
                            }
                        }
                        KeyCode::Tab => {
                            if self.editing {
                                self.focus_next();
                                handled = true;
                            }
                        }
                        KeyCode::BackTab => {
                            if self.editing {
                                self.focus_previous();
                                handled = true;
                            }
                        }
                        KeyCode::Right => {
                            if self.editing && self.theme_state.borrow().editing {
                                self.next_color();
                                handled = true;
                            }
                        }
                        KeyCode::Left => {
                            if self.editing && self.theme_state.borrow().editing {
                                self.previous_color();
                                handled = true;
                            }
                        }
                        KeyCode::Enter => {
                            if self.editing {
                                self.set_config(state);
                                self.reset_input_state();
                                self.focus = Focus::SSHUser;
                                self.editing = false;
                                handled = true;
                            }
                        }
                        KeyCode::Backspace => {
                            if self.editing && !self.theme_state.borrow().editing {
                                self.pop_input_char();
                                handled = true;
                            }
                        }
                        KeyCode::Char(c) => {
                            if self.editing && !self.theme_state.borrow().editing {
                                // handle value update for focused element
                                self.push_input_char(c);
                                handled = true;
                            } else if c == 'c' && !self.editing {
                                // enter edit mode
                                self.editing = true;
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
