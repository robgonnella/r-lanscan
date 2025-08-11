use crate::ui::{
    components::{
        header::Header,
        input::{Input, InputState},
        popover::get_popover_area,
    },
    events::types::{Command, Event},
    store::{
        Store,
        action::Action,
        derived::get_selected_device_config_from_state,
        state::{State, ViewID},
    },
};
use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event as CrossTermEvent, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget, Wrap},
};
use std::{cell::RefCell, sync::Arc};

use super::traits::{
    CustomStatefulWidget, CustomWidget, CustomWidgetContext, CustomWidgetRef, EventHandler, View,
};

#[derive(Debug, Clone)]
enum Focus {
    SSHUser,
    SSHPort,
    SSHIdentity,
    BrowserPort,
}

pub struct DeviceView {
    store: Arc<Store>,
    editing: RefCell<bool>,
    focus: RefCell<Focus>,
    ssh_user_state: RefCell<InputState>,
    ssh_port_state: RefCell<InputState>,
    ssh_identity_state: RefCell<InputState>,
    browser_port_state: RefCell<InputState>,
}

impl DeviceView {
    pub fn new(store: Arc<Store>) -> Self {
        Self {
            store,
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
            browser_port_state: RefCell::new(InputState {
                editing: false,
                value: String::from(""),
            }),
        }
    }

    fn render_device_ssh_config(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if !*self.editing.borrow() || self.browser_port_state.borrow().editing {
            let device_config = get_selected_device_config_from_state(&ctx.state);
            self.ssh_user_state.borrow_mut().value = device_config.ssh_user.clone();
            self.ssh_port_state.borrow_mut().value = device_config.ssh_port.to_string();
            self.ssh_identity_state.borrow_mut().value = device_config.ssh_identity_file.clone();
        }

        let rects = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Length(1), // spacer
            Constraint::Length(1), // user
            Constraint::Length(1), // spacer
            Constraint::Length(1), // port
            Constraint::Length(1), // spacer
            Constraint::Length(1), // identity
            Constraint::Length(1), // spacer
        ])
        .split(area);

        let header = Header::new("Device SSH Config".to_string());
        let ssh_user_input = Input::new("SSH User");
        let ssh_port_input = Input::new("SSH Port");
        let ssh_identity_input = Input::new("SSH Identity");

        header.render(rects[0], buf, ctx);
        ssh_user_input.render(rects[2], buf, &mut self.ssh_user_state.borrow_mut(), ctx);
        ssh_port_input.render(rects[4], buf, &mut self.ssh_port_state.borrow_mut(), ctx);
        ssh_identity_input.render(
            rects[6],
            buf,
            &mut self.ssh_identity_state.borrow_mut(),
            ctx,
        );
    }

    fn render_device_info(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        device: &DeviceWithPorts,
        ctx: &CustomWidgetContext,
    ) {
        let [header_area, _, info_area] = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // info
        ])
        .areas(area);

        let host_str = format!("Hostname: {0}", device.hostname);
        let ip_str = if device.is_current_host {
            format!("IP: {0} [YOU]", device.ip)
        } else {
            format!("IP: {0}", device.ip)
        };
        let mac_str = format!("MAC: {0}", device.mac);
        let vendor_str = format!("Vendor: {0}", device.vendor);
        let open_ports_str = format!(
            "Open Ports: {0}",
            device
                .open_ports
                .iter()
                .sorted_by_key(|p| p.id)
                .map(|p| p.id.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );

        let [
            host_area,
            _,
            ip_area,
            _,
            mac_area,
            _,
            vendor_area,
            _,
            ports_area,
        ] = Layout::vertical([
            Constraint::Length(1),       // hostname
            Constraint::Length(1),       // spacer
            Constraint::Length(1),       // ip
            Constraint::Length(1),       // spacer
            Constraint::Length(1),       // mac
            Constraint::Length(1),       // spacer
            Constraint::Length(1),       // vendor
            Constraint::Length(1),       // spacer
            Constraint::Percentage(100), // ports
        ])
        .areas(info_area);

        let header = Header::new("Device Info".to_string());
        let host = Line::from(host_str);
        let ip = Line::from(ip_str);
        let mac = Line::from(mac_str);
        let vendor = Line::from(vendor_str);
        let open_ports = Paragraph::new(vec![Line::from(open_ports_str)]).wrap(Wrap { trim: true });
        header.render(header_area, buf, ctx);
        host.render(host_area, buf);
        ip.render(ip_area, buf);
        mac.render(mac_area, buf);
        vendor.render(vendor_area, buf);
        open_ports.render(ports_area, buf);
    }

    fn render_cmd_output(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if self.is_tracing(&ctx.state) {
            let [label_area] = Layout::vertical([
                Constraint::Min(1), // label
            ])
            .areas(area);

            let header = Header::new("tracing...".to_string());
            header.render(label_area, buf, ctx);
        } else if ctx.state.cmd_output.is_some() {
            let (cmd, output) = ctx.state.cmd_output.clone().unwrap();
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

            let [label_area, _, status_area, _, stderr_area, _, stdout_area] = Layout::vertical([
                Constraint::Length(1),       // label
                Constraint::Length(1),       // spacer
                Constraint::Length(1),       // status
                Constraint::Length(1),       // spacer
                Constraint::Min(1),          // stderr
                Constraint::Length(1),       // spacer
                Constraint::Percentage(100), // stdout
            ])
            .areas(area);

            header.render(label_area, buf, ctx);
            status.render(status_area, buf);
            stderr.render(stderr_area, buf);
            stdout.render(stdout_area, buf);
        }
    }

    fn render_browser_port_select_popover(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if *self.editing.borrow() && self.browser_port_state.borrow().editing {
            let block = Block::bordered()
                .border_type(BorderType::Double)
                .border_style(Style::new().fg(ctx.state.colors.border_color))
                .padding(Padding::uniform(2))
                .style(Style::new().bg(ctx.state.colors.buffer_bg));
            let inner_area = block.inner(area);
            let [header_area, _, port_area, message_area] = Layout::vertical([
                Constraint::Length(1),       // header
                Constraint::Length(1),       // spacer
                Constraint::Percentage(100), // port select
                Constraint::Length(1),       // enter to submit message
            ])
            .areas(inner_area);

            let header = Header::new("Enter port to browse".to_string());
            let input = Input::new("Port");
            let message = Paragraph::new("Press enter to open browser or esc to cancel").centered();

            Clear.render(area, buf);
            block.render(area, buf);
            header.render(header_area, buf, ctx);
            input.render(
                port_area,
                buf,
                &mut self.browser_port_state.borrow_mut(),
                ctx,
            );
            message.render(message_area, buf);
        }
    }

    fn push_input_char(&self, char: char) {
        match *self.focus.borrow() {
            Focus::BrowserPort => self.browser_port_state.borrow_mut().value.push(char),
            Focus::SSHUser => self.ssh_user_state.borrow_mut().value.push(char),
            Focus::SSHPort => self.ssh_port_state.borrow_mut().value.push(char),
            Focus::SSHIdentity => self.ssh_identity_state.borrow_mut().value.push(char),
        };
    }

    fn pop_input_char(&self) {
        match *self.focus.borrow() {
            Focus::BrowserPort => self.browser_port_state.borrow_mut().value.pop(),
            Focus::SSHUser => self.ssh_user_state.borrow_mut().value.pop(),
            Focus::SSHPort => self.ssh_port_state.borrow_mut().value.pop(),
            Focus::SSHIdentity => self.ssh_identity_state.borrow_mut().value.pop(),
        };
    }

    fn reset_input_state(&self) {
        self.browser_port_state.borrow_mut().editing = false;
        self.ssh_user_state.borrow_mut().editing = false;
        self.ssh_port_state.borrow_mut().editing = false;
        self.ssh_identity_state.borrow_mut().editing = false;
        *self.focus.borrow_mut() = Focus::SSHUser;
        *self.editing.borrow_mut() = false;
    }

    fn focus_next(&self) {
        let next_focus = match *self.focus.borrow() {
            Focus::BrowserPort => {
                if *self.editing.borrow() {
                    self.browser_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::BrowserPort
            }
            Focus::SSHUser => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                }
                Focus::SSHPort
            }
            Focus::SSHPort => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                }
                Focus::SSHIdentity
            }
            Focus::SSHIdentity => {
                if *self.editing.borrow() {
                    self.ssh_user_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                }
                Focus::SSHUser
            }
        };

        *self.focus.borrow_mut() = next_focus;
    }

    fn focus_previous(&self) {
        let next_focus = match *self.focus.borrow() {
            Focus::BrowserPort => {
                if *self.editing.borrow() {
                    self.browser_port_state.borrow_mut().editing = true;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::BrowserPort
            }
            Focus::SSHIdentity => {
                if *self.editing.borrow() {
                    self.browser_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = true;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHPort
            }
            Focus::SSHPort => {
                if *self.editing.borrow() {
                    self.browser_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = false;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = true;
                }
                Focus::SSHUser
            }
            Focus::SSHUser => {
                if *self.editing.borrow() {
                    self.browser_port_state.borrow_mut().editing = false;
                    self.ssh_identity_state.borrow_mut().editing = true;
                    self.ssh_port_state.borrow_mut().editing = false;
                    self.ssh_user_state.borrow_mut().editing = false;
                }
                Focus::SSHIdentity
            }
        };

        *self.focus.borrow_mut() = next_focus;
    }

    fn is_tracing(&self, state: &State) -> bool {
        if state.cmd_in_progress.is_some() {
            let cmd = state.cmd_in_progress.clone().unwrap();
            matches!(cmd, Command::TraceRoute(_))
        } else {
            false
        }
    }
}

impl View for DeviceView {
    fn id(&self) -> ViewID {
        ViewID::Device
    }
    fn legend(&self, state: &State) -> &str {
        if *self.editing.borrow() {
            "(esc) exit configuration | (enter) save configuration"
        } else if self.is_tracing(state) {
            "tracing..."
        } else {
            "(esc) back to devices | (c) configure | (s) SSH | (t) traceroute | (b) browse"
        }
    }
    fn override_main_legend(&self, _state: &State) -> bool {
        true
    }
}

impl CustomWidgetRef for DeviceView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let [left_area, right_area] = Layout::horizontal([
            Constraint::Percentage(50), // info
            Constraint::Percentage(50), // command output
        ])
        .areas(area);

        let [ssh_area, _, info_area] = Layout::vertical([
            Constraint::Length(8),       // ssh info
            Constraint::Length(1),       // spacer
            Constraint::Percentage(100), // device info
        ])
        .areas(left_area);

        let popover_area = get_popover_area(ctx.app_area, 40, 30);

        if let Some(device) = ctx.state.selected_device.clone() {
            self.render_device_ssh_config(ssh_area, buf, ctx);
            self.render_device_info(info_area, buf, &device, ctx);
            self.render_cmd_output(right_area, buf, ctx);
            // important that this is last so it properly layers on top
            self.render_browser_port_select_popover(popover_area, buf, ctx);
        }
    }
}

impl EventHandler for DeviceView {
    fn process_event(&self, evt: &CrossTermEvent, ctx: &CustomWidgetContext) -> bool {
        if ctx.state.render_view_select {
            return false;
        }

        let mut handled = false;

        match evt {
            CrossTermEvent::FocusGained => {}
            CrossTermEvent::FocusLost => {}
            CrossTermEvent::Mouse(_m) => {}
            CrossTermEvent::Paste(_s) => {}
            CrossTermEvent::Resize(_x, _y) => {}
            CrossTermEvent::Key(key) => match key.code {
                KeyCode::Esc => {
                    if *self.editing.borrow() {
                        self.reset_input_state();
                        handled = true;
                    } else if !self.is_tracing(&ctx.state) {
                        self.store.dispatch(Action::ClearCommandOutput);
                        self.store.dispatch(Action::UpdateView(ViewID::Devices));
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
                KeyCode::Enter => {
                    if *self.editing.borrow() {
                        if self.browser_port_state.borrow().editing {
                            let port_str = self.browser_port_state.borrow().value.clone();
                            if let Ok(port) = port_str.parse::<u16>() {
                                let _ = ctx.events.send(Event::ExecCommand(Command::Browse(
                                    ctx.state.selected_device.clone().unwrap().into(),
                                    port,
                                )));
                                self.reset_input_state();
                                handled = true;
                            }
                        } else if let Some(device) = ctx.state.selected_device.clone() {
                            // save config
                            let mut device_config =
                                get_selected_device_config_from_state(&ctx.state);
                            device_config.ssh_user = self.ssh_user_state.borrow().value.clone();
                            let port = self.ssh_port_state.borrow().value.clone().parse::<u16>();
                            device_config.ssh_port = port.unwrap_or(22);
                            device_config.ssh_identity_file =
                                self.ssh_identity_state.borrow().value.clone();
                            self.store
                                .dispatch(Action::UpdateDeviceConfig(device_config));
                            self.store
                                .dispatch(Action::UpdateSelectedDevice(device.mac));
                            self.reset_input_state();
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
                    } else if self.is_tracing(&ctx.state) {
                        handled = true
                    } else if c == 'c' {
                        // enter edit mode
                        *self.focus.borrow_mut() = Focus::SSHUser;
                        self.ssh_user_state.borrow_mut().editing = true;
                        *self.editing.borrow_mut() = true;
                        handled = true;
                    } else if c == 's' {
                        if ctx.state.cmd_in_progress.is_none() {
                            handled = true;
                            let _ = ctx.events.send(Event::ExecCommand(Command::Ssh(
                                ctx.state.selected_device.clone().unwrap().into(),
                                ctx.state.selected_device_config.clone().unwrap(),
                            )));
                        }
                    } else if c == 't' {
                        if !self.is_tracing(&ctx.state) {
                            let _ = ctx.events.send(Event::ExecCommand(Command::TraceRoute(
                                ctx.state.selected_device.clone().unwrap().into(),
                            )));
                            handled = true;
                        }
                    } else if c == 'b' {
                        *self.focus.borrow_mut() = Focus::BrowserPort;
                        let mut browser_state = self.browser_port_state.borrow_mut();
                        browser_state.editing = true;
                        browser_state.value = "80".to_string();
                        *self.editing.borrow_mut() = true;
                        handled = true;
                    }
                }
                _ => {}
            },
        }

        handled
    }
}

#[cfg(test)]
#[path = "./device_tests.rs"]
mod tests;
