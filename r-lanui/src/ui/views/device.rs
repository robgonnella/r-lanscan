use crate::ui::{
    components::{
        device_info::DeviceInfo,
        header::Header,
        input::{Input, InputState},
    },
    store::{
        action::Action,
        state::{Command, State, ViewID},
        store::Store,
    },
};
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget, WidgetRef, Wrap},
};
use std::{cell::RefCell, sync::Arc};

use super::traits::{CustomWidget, EventHandler, View};

#[derive(Debug, Clone)]
enum SSHFocus {
    User,
    Port,
    Identity,
}

pub struct DeviceView {
    store: Arc<Store>,
    editing: RefCell<bool>,
    focus: RefCell<SSHFocus>,
    ssh_user_state: RefCell<InputState>,
    ssh_port_state: RefCell<InputState>,
    ssh_identity_state: RefCell<InputState>,
}

impl DeviceView {
    pub fn new(store: Arc<Store>) -> Self {
        Self {
            store,
            editing: RefCell::new(false),
            focus: RefCell::new(SSHFocus::User),
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
        }
    }

    fn render_device_ssh_config(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        device: &DeviceWithPorts,
        state: &State,
    ) {
        if !*self.editing.borrow() {
            let device_config = self.store.get_device_config_from_state(device, state);
            self.ssh_user_state.borrow_mut().value = device_config.ssh_user.clone();
            self.ssh_port_state.borrow_mut().value = device_config.ssh_port.to_string();
            self.ssh_identity_state.borrow_mut().value = device_config.ssh_identity_file.clone();
        }

        let rects = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        let ssh_user_input = Input::new("SSH User");
        let ssh_port_input = Input::new("SSH Port");
        let ssh_identity_input = Input::new("SSH Identity");

        ssh_user_input.render(rects[0], buf, &mut self.ssh_user_state.borrow_mut());
        ssh_port_input.render(rects[1], buf, &mut self.ssh_port_state.borrow_mut());
        ssh_identity_input.render(rects[2], buf, &mut self.ssh_identity_state.borrow_mut());
    }

    fn render_device_info(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        device: &DeviceWithPorts,
        state: &State,
    ) {
        DeviceInfo::new(device.clone()).render(area, buf, state);
    }

    fn render_cmd_output(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        if state.cmd_output.is_some() {
            let (cmd, output) = state.cmd_output.clone().unwrap();
            let header = Header::new(cmd.to_string());

            let status_value = Span::from(output.status.to_string());
            let status = Line::from(vec![status_value]);

            let stderr_label = Span::from("stderr: ");
            let stderr_value = Span::from(String::from_utf8(output.stderr).unwrap());
            let stderr = Paragraph::new(Line::from(vec![stderr_label, stderr_value]))
                .wrap(Wrap { trim: true });

            let stdout_label = Span::from("stdout: ");
            let stdout_value = Span::from(String::from_utf8(output.stdout).unwrap());

            let stdout = Paragraph::new(Line::from(vec![stdout_label, stdout_value]))
                .wrap(Wrap { trim: true });

            let rects = Layout::vertical([
                Constraint::Length(1),       // label
                Constraint::Length(1),       // spacer
                Constraint::Length(1),       // status
                Constraint::Length(1),       // spacer
                Constraint::Min(1),          // stderr
                Constraint::Length(1),       // spacer
                Constraint::Percentage(100), // stdout
            ])
            .split(area);

            header.render(rects[0], buf, &state);
            status.render(rects[2], buf);
            stderr.render(rects[4], buf);
            stdout.render(rects[6], buf);
        }
    }

    fn push_input_char(&self, char: char) {
        match *self.focus.borrow() {
            SSHFocus::User => self.ssh_user_state.borrow_mut().value.push(char),
            SSHFocus::Port => self.ssh_port_state.borrow_mut().value.push(char),
            SSHFocus::Identity => self.ssh_identity_state.borrow_mut().value.push(char),
        };
    }

    fn pop_input_char(&self) {
        match *self.focus.borrow() {
            SSHFocus::User => self.ssh_user_state.borrow_mut().value.pop(),
            SSHFocus::Port => self.ssh_port_state.borrow_mut().value.pop(),
            SSHFocus::Identity => self.ssh_identity_state.borrow_mut().value.pop(),
        };
    }

    fn reset_input_state(&self) {
        self.ssh_user_state.borrow_mut().editing = false;
        self.ssh_port_state.borrow_mut().editing = false;
        self.ssh_identity_state.borrow_mut().editing = false;
    }

    fn focus_next(&self) {
        let next_focus = match *self.focus.borrow() {
            SSHFocus::User => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                }
                SSHFocus::Port
            }
            SSHFocus::Port => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                }
                SSHFocus::Identity
            }
            SSHFocus::Identity => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                }
                SSHFocus::User
            }
        };

        *self.focus.borrow_mut() = next_focus;
    }

    fn focus_previous(&self) {
        let next_focus = match *self.focus.borrow() {
            SSHFocus::Identity => {
                if *self.editing.borrow() {
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                SSHFocus::Port
            }
            SSHFocus::Port => {
                if *self.editing.borrow() {
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = true;
                }
                SSHFocus::User
            }
            SSHFocus::User => {
                if *self.editing.borrow() {
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                SSHFocus::Identity
            }
        };

        *self.focus.borrow_mut() = next_focus;
    }
}

impl View for DeviceView {
    fn id(&self) -> ViewID {
        ViewID::Device
    }
    fn legend(&self) -> &str {
        if *self.editing.borrow() {
            "(esc) exit configuration | (enter) save configuration"
        } else {
            "(esc) back to devices | (c) configure | (s) SSH | (t) traceroute"
        }
    }
    fn override_main_legend(&self) -> bool {
        true
    }
}

impl WidgetRef for DeviceView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.store.get_state();

        let view_rects = Layout::horizontal([
            Constraint::Percentage(50), // info
            Constraint::Percentage(50), // command output
        ])
        .split(area);

        let info_rects = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacer
            Constraint::Length(3), // ssh info
            Constraint::Min(5),    // device info
        ])
        .split(view_rects[0]);

        let label_rects = Layout::horizontal([Constraint::Length(20)]).split(info_rects[0]);

        let header = Header::new(String::from("Device Info"));

        header.render(label_rects[0], buf, &state);

        if let Some(device) = state.selected_device.clone() {
            self.render_device_ssh_config(info_rects[2], buf, &device, &state);
            self.render_device_info(info_rects[3], buf, &device, &state);
            self.render_cmd_output(view_rects[1], buf, &state);
        }
    }
}

impl EventHandler for DeviceView {
    fn process_event(&self, evt: &Event, state: &State) -> bool {
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
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    if *self.editing.borrow() {
                        self.reset_input_state();
                        *self.focus.borrow_mut() = SSHFocus::User;
                        *self.editing.borrow_mut() = false;
                    } else {
                        self.store.dispatch(Action::UpdateView(ViewID::Devices));
                    }
                    handled = true;
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
                KeyCode::Enter => {
                    if *self.editing.borrow() {
                        // save config
                        if let Some(device) = state.selected_device.clone() {
                            let mut device_config =
                                self.store.get_device_config_from_state(&device, &state);
                            device_config.ssh_user = self.ssh_user_state.borrow().value.clone();
                            let mut port =
                                self.ssh_port_state.borrow().value.clone().parse::<u16>();
                            if port.is_err() {
                                port = Ok(22);
                            }
                            device_config.ssh_port = port.unwrap();
                            device_config.ssh_identity_file =
                                self.ssh_identity_state.borrow().value.clone();
                            self.store
                                .dispatch(Action::UpdateDeviceConfig(device_config));
                            self.reset_input_state();
                            *self.focus.borrow_mut() = SSHFocus::User;
                            *self.editing.borrow_mut() = false;
                            handled = true;
                        }
                    }
                }
                KeyCode::Backspace => {
                    if *self.editing.borrow() {
                        self.pop_input_char();
                        handled = true;
                    }
                }
                KeyCode::Char(c) => {
                    if *self.editing.borrow() {
                        // handle value update for focused element
                        self.push_input_char(c);
                        handled = true;
                    } else if c == 'c' {
                        // enter edit mode
                        *self.editing.borrow_mut() = true;
                        self.ssh_user_state.borrow_mut().editing = true;
                        handled = true;
                    } else if c == 's' {
                        if state.execute_cmd.is_none() {
                            handled = true;
                            self.store.dispatch(Action::ExecuteCommand(Command::SSH));
                        }
                    } else if c == 't' {
                        if state.execute_cmd.is_none() {
                            handled = true;
                            self.store
                                .dispatch(Action::ExecuteCommand(Command::TRACEROUTE));
                        }
                    }
                }
                _ => {}
            },
        }

        handled
    }
}
