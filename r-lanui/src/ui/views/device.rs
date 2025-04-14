use crate::{
    config::DeviceConfig,
    ui::{
        components::{
            device_info::DeviceInfo,
            header::Header,
            input::{Input, InputState},
        },
        store::{
            action::Action,
            dispatcher::Dispatcher,
            state::{State, ViewID},
        },
    },
};
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Layout, Rect},
    widgets::{StatefulWidget, WidgetRef},
};
use std::{cell::RefCell, sync::Arc};

use super::{CustomWidget, EventHandler, View};

#[derive(Debug, Clone)]
enum SSHFocus {
    User,
    Port,
    Identity,
}

pub struct DeviceView {
    dispatcher: Arc<Dispatcher>,
    editing: bool,
    focus: SSHFocus,
    ssh_user_state: RefCell<InputState>,
    ssh_port_state: RefCell<InputState>,
    ssh_identity_state: RefCell<InputState>,
}

impl DeviceView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        Self {
            dispatcher,
            editing: false,
            focus: SSHFocus::User,
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
        if !self.editing {
            let device_config = self.get_device_config_from_state(device, state);
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

    fn push_input_char(&self, char: char) {
        match self.focus {
            SSHFocus::User => self.ssh_user_state.borrow_mut().value.push(char),
            SSHFocus::Port => self.ssh_port_state.borrow_mut().value.push(char),
            SSHFocus::Identity => self.ssh_identity_state.borrow_mut().value.push(char),
        };
    }

    fn pop_input_char(&self) {
        match self.focus {
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

    fn focus_next(&mut self) {
        let next_focus = match self.focus {
            SSHFocus::User => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                }
                SSHFocus::Port
            }
            SSHFocus::Port => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                }
                SSHFocus::Identity
            }
            SSHFocus::Identity => {
                if self.editing {
                    self.ssh_user_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                }
                SSHFocus::User
            }
        };

        self.focus = next_focus;
    }

    fn focus_previous(&mut self) {
        let next_focus = match self.focus {
            SSHFocus::Identity => {
                if self.editing {
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                SSHFocus::Port
            }
            SSHFocus::Port => {
                if self.editing {
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = true;
                }
                SSHFocus::User
            }
            SSHFocus::User => {
                if self.editing {
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                SSHFocus::Identity
            }
        };

        self.focus = next_focus;
    }

    fn get_device_config_from_state(
        &self,
        device: &DeviceWithPorts,
        state: &State,
    ) -> DeviceConfig {
        let device_config: DeviceConfig;

        if state.config.device_configs.contains_key(&device.ip) {
            device_config = state.config.device_configs.get(&device.ip).unwrap().clone();
        } else if state.config.device_configs.contains_key(&device.mac) {
            device_config = state
                .config
                .device_configs
                .get(&device.mac)
                .unwrap()
                .clone();
        } else {
            device_config = DeviceConfig {
                id: device.mac.clone(),
                ssh_identity_file: state.config.default_ssh_identity.clone(),
                ssh_port: state
                    .config
                    .default_ssh_port
                    .clone()
                    .parse::<u16>()
                    .unwrap(),
                ssh_user: state.config.default_ssh_user.clone(),
            }
        }

        device_config
    }
}

impl View for DeviceView {
    fn id(&self) -> ViewID {
        ViewID::Device
    }
    fn legend(&self) -> &str {
        if self.editing {
            "(esc) exit configuration | (enter) save configuration"
        } else {
            "(esc) back to devices | (c) configure | (s) SSH"
        }
    }
    fn override_main_legend(&self) -> bool {
        true
    }
}

impl WidgetRef for DeviceView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        let view_rects = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(area);

        let label_rects = Layout::horizontal([Constraint::Length(20)]).split(view_rects[0]);

        let header = Header::new(String::from("Device Info"));

        header.render(label_rects[0], buf, &state);

        if let Some(selected) = state.selected_device.clone() {
            if let Some(device) = state.device_map.get(&selected) {
                self.render_device_ssh_config(view_rects[2], buf, device, &state);
                self.render_device_info(view_rects[3], buf, device, &state);
            }
        }
    }
}

impl EventHandler for DeviceView {
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
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    if self.editing {
                        self.reset_input_state();
                        self.focus = SSHFocus::User;
                        self.editing = false;
                    } else {
                        self.dispatcher
                            .dispatch(Action::UpdateView(ViewID::Devices));
                    }
                    handled = true;
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
                KeyCode::Enter => {
                    if self.editing {
                        // save config
                        if let Some(selected) = state.selected_device.clone() {
                            if let Some(device) = state.device_map.get(&selected) {
                                let mut device_config =
                                    self.get_device_config_from_state(device, state);
                                device_config.ssh_user = self.ssh_user_state.borrow().value.clone();
                                let mut port =
                                    self.ssh_port_state.borrow().value.clone().parse::<u16>();
                                if port.is_err() {
                                    port = Ok(22);
                                }
                                device_config.ssh_port = port.unwrap();
                                device_config.ssh_identity_file =
                                    self.ssh_identity_state.borrow().value.clone();
                                self.dispatcher
                                    .dispatch(Action::UpdateDeviceConfig(device_config));
                                self.reset_input_state();
                                self.focus = SSHFocus::User;
                                self.editing = false;
                                handled = true;
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    if self.editing {
                        self.pop_input_char();
                        handled = true;
                    }
                }
                KeyCode::Char(c) => {
                    if self.editing {
                        // handle value update for focused element
                        self.push_input_char(c);
                        handled = true;
                    } else if c == 'c' {
                        // enter edit mode
                        self.editing = true;
                        self.ssh_user_state.borrow_mut().editing = true;
                        handled = true;
                    } else if c == 's' {
                        if !state.paused {
                            handled = true;
                            self.dispatcher.dispatch(Action::TogglePause);
                        }
                    }
                }
                _ => {}
            },
        }

        handled
    }
}
