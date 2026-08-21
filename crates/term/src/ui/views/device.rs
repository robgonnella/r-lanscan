//! Single device detail view with SSH config, info, and command output.

use crate::{
    config::DeviceConfig,
    ipc::message::{Command, MainMessage},
    shell::traits::BrowseArgs,
    store::{action::Action, state::State},
    ui::{
        components::{
            input::{Input, InputState},
            label::Label,
            popover::{
                base::Popover, browse::BrowsePopover, simple::SimplePopover,
            },
        },
        views::traits::CustomEventContext,
    },
};
use color_eyre::eyre::Result;
use r_lanlib::scanners::Device;
use ratatui::{
    crossterm::event::{Event as CrossTermEvent, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, RenderDirection, Sparkline, Widget, Wrap},
};
use std::{cell::RefCell, rc::Rc};

use super::traits::{
    CustomStatefulWidget, CustomWidget, CustomWidgetContext, CustomWidgetRef,
    EventHandler, View,
};

/// Tracks which SSH input field currently has focus.
#[derive(Debug, Clone)]
enum FocusSsh {
    User,
    Port,
    Identity,
}

/// Tracks which Browser input field currently has focus.
#[derive(Debug, Clone)]
enum FocusBrowser {
    Select,
    Port,
}

const FOCUS_SSH_ARRAY: [FocusSsh; 3] =
    [FocusSsh::User, FocusSsh::Port, FocusSsh::Identity];

const FOCUS_BROWSER_ARRAY: [FocusBrowser; 2] =
    [FocusBrowser::Select, FocusBrowser::Port];

/// View for displaying device details and executing SSH, traceroute, browse.
pub struct DeviceView {
    device: Device,
    device_config: DeviceConfig,
    editing_ssh: RefCell<bool>,
    editing_browser: RefCell<bool>,
    ssh_focus: RefCell<i8>,
    browser_focus: RefCell<i8>,
    ssh_user_state: RefCell<InputState>,
    ssh_port_state: RefCell<InputState>,
    ssh_identity_state: RefCell<InputState>,
    browser_port_state: Rc<RefCell<InputState>>,
    browser_select_state: Rc<RefCell<InputState>>,
    confirm_config_removal: RefCell<bool>,
}

impl DeviceView {
    /// Creates a new device view with the given dispatcher.
    pub fn new(device: Device, device_config: DeviceConfig) -> Self {
        Self {
            device,
            device_config,
            confirm_config_removal: RefCell::new(false),
            editing_ssh: RefCell::new(false),
            editing_browser: RefCell::new(false),
            ssh_focus: RefCell::new(0),
            browser_focus: RefCell::new(0),
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
            browser_port_state: Rc::new(RefCell::new(InputState {
                editing: false,
                value: String::from("80"),
            })),
            browser_select_state: Rc::new(RefCell::new(InputState {
                editing: false,
                value: String::from("default"),
            })),
        }
    }

    pub fn update_device(
        &mut self,
        device: Device,
        device_config: DeviceConfig,
    ) {
        self.device = device;
        self.device_config = device_config;
    }

    fn render_device_ssh_config(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if !*self.editing_ssh.borrow() {
            self.ssh_user_state.borrow_mut().value =
                self.device_config.ssh_user.clone();
            self.ssh_port_state.borrow_mut().value =
                self.device_config.ssh_port.to_string();
            self.ssh_identity_state.borrow_mut().value =
                self.device_config.ssh_identity_file.clone();
        }

        let [label_area, _, user_area, _, port_area, _, identity_area, _] =
            Layout::vertical([
                Constraint::Length(1), // label
                Constraint::Length(1), // spacer
                Constraint::Length(1), // user
                Constraint::Length(1), // spacer
                Constraint::Length(1), // port
                Constraint::Length(1), // spacer
                Constraint::Length(1), // identity
                Constraint::Length(1), // spacer
            ])
            .areas(area);

        let label = Label::new("SSH Config".to_string());
        let ssh_user_input = Input::new("User");
        let ssh_port_input = Input::new("Port");
        let ssh_identity_input = Input::new("Identity");

        label.render(label_area, buf, ctx);

        ssh_user_input.render(
            user_area,
            buf,
            &mut self.ssh_user_state.borrow_mut(),
            ctx,
        );
        ssh_port_input.render(
            port_area,
            buf,
            &mut self.ssh_port_state.borrow_mut(),
            ctx,
        );
        ssh_identity_input.render(
            identity_area,
            buf,
            &mut self.ssh_identity_state.borrow_mut(),
            ctx,
        );
    }

    fn render_device_info(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let [label_area, _, info_area] = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // info
        ])
        .areas(area);

        let host_str = format!("Hostname: {0}", self.device.hostname);
        let ip_str = if self.device.is_current_host {
            format!("IP: {0} [YOU]", self.device.ip)
        } else if self.device.is_gateway {
            format!("IP: {0} [GTWY]", self.device.ip)
        } else {
            format!("IP: {0}", self.device.ip)
        };
        let mac_str = format!("MAC: {0}", self.device.mac);
        let vendor_str = format!("Vendor: {0}", self.device.vendor);
        let open_ports_str = format!(
            "Open Ports: {0}",
            self.device
                .open_ports
                .to_sorted_vec()
                .iter()
                .map(|p| p.to_string())
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

        let label = Label::new("Info".to_string());
        let host = Line::from(host_str);
        let ip = Line::from(ip_str);
        let mac = Line::from(mac_str);
        let vendor = Line::from(vendor_str);
        let open_ports = Paragraph::new(vec![Line::from(open_ports_str)])
            .wrap(Wrap { trim: true });
        label.render(label_area, buf, ctx);
        host.render(host_area, buf);
        ip.render(ip_area, buf);
        mac.render(mac_area, buf);
        vendor.render(vendor_area, buf);
        open_ports.render(ports_area, buf);
    }

    fn render_latency_sparkline(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        // Constrain to a fixed width so the sparkline doesn't stretch
        // across the whole right column.
        const SPARK_WIDTH: u16 = 30;

        let [constrained, _] = Layout::horizontal([
            Constraint::Length(SPARK_WIDTH),
            Constraint::Min(0),
        ])
        .areas(area);

        let [label_area, _, spark_area, _] = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacer
            Constraint::Length(4), // sparkline bars
            Constraint::Min(0),    // remaining space (ignored)
        ])
        .areas(constrained);

        // Reverse so newest sample is first — combined with RightToLeft
        // rendering this puts the newest bar on the right and ensures the
        // widget always shows the most recent data when history is longer
        // than the widget width.
        let history: Vec<u64> = ctx
            .state
            .latency_history
            .get(&self.device.ip)
            .map(|h| h.iter().copied().rev().collect())
            .unwrap_or_default();

        let latest_str = history
            .first()
            .map(|v| format!("Latency: {}ms", v))
            .unwrap_or_else(|| "Latency (ms)".to_string());

        Label::new(latest_str)
            .width(SPARK_WIDTH)
            .render(label_area, buf, ctx);

        // Scale relative to the highest observed value. Floor at 1 only to
        // avoid divide-by-zero; sparklines show relative change, not absolute.
        let max = history.iter().copied().max().unwrap_or(1).max(1);

        Sparkline::default()
            .data(&history)
            .max(max)
            .direction(RenderDirection::RightToLeft)
            .style(
                Style::default()
                    .fg(ctx.state.colors.border_color)
                    .bg(ctx.state.colors.buffer_bg),
            )
            .render(spark_area, buf);
    }

    fn render_cmd_output(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if !self.is_tracing(ctx.state)
            && let Some((cmd, output)) = ctx.state.cmd_output.as_ref()
        {
            let label = Label::new(cmd.to_string());

            let status_value = Span::from(output.status.to_string());
            let status = Line::from(vec![status_value]);

            let stderr_label = Span::from("stderr: ");
            let stderr_value = Span::from(
                String::from_utf8(output.stderr.clone()).unwrap_or_default(),
            );
            let stderr =
                Paragraph::new(Line::from(vec![stderr_label, stderr_value]))
                    .wrap(Wrap { trim: true });

            let stdout_label = Span::from("stdout: ");
            let stdout_value = Span::from(
                String::from_utf8(output.stdout.clone()).unwrap_or_default(),
            );

            let stdout =
                Paragraph::new(Line::from(vec![stdout_label, stdout_value]))
                    .wrap(Wrap { trim: true });

            let [label_area, _, status_area, _, stderr_area, _, stdout_area] =
                Layout::vertical([
                    Constraint::Length(1),       // label
                    Constraint::Length(1),       // spacer
                    Constraint::Length(1),       // status
                    Constraint::Length(1),       // spacer
                    Constraint::Min(1),          // stderr
                    Constraint::Length(1),       // spacer
                    Constraint::Percentage(100), // stdout
                ])
                .areas(area);

            label.render(label_area, buf, ctx);
            status.render(status_area, buf);
            stderr.render(stderr_area, buf);
            stdout.render(stdout_area, buf);
        }
    }

    fn next_browser(&self) {
        let mut next = "default".to_string();
        if self.browser_select_state.borrow().value == "default" {
            next = "lynx".to_string();
        }
        self.browser_select_state.borrow_mut().value = next;
    }

    fn render_browser_config_popover(
        &self,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        if *self.editing_browser.borrow() {
            let browse = BrowsePopover::new(
                self.browser_select_state.clone(),
                self.browser_port_state.clone(),
            );
            Popover::new(&browse).width(40).height(30).render_ref(
                ctx.app_area,
                buf,
                ctx,
            )?;
        }

        Ok(())
    }

    fn render_config_removal_popover(
        &self,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        if *self.confirm_config_removal.borrow() {
            let simple = SimplePopover::new(
                "Are you sure you want to reset the config for this device?",
            )
            .footer("Enter to Confirm | Esc to Cancel")
            .message_centered()
            .footer_centered();

            Popover::new(&simple).height(25).render_ref(
                ctx.app_area,
                buf,
                ctx,
            )?;
        }

        Ok(())
    }

    fn push_ssh_input_char(&self, char: char) {
        let idx = *self.ssh_focus.borrow();
        let focus = FOCUS_SSH_ARRAY[idx as usize].clone();
        match focus {
            FocusSsh::User => self.ssh_user_state.borrow_mut().value.push(char),
            FocusSsh::Port => self.ssh_port_state.borrow_mut().value.push(char),
            FocusSsh::Identity => {
                self.ssh_identity_state.borrow_mut().value.push(char)
            }
        };
    }

    fn push_browser_input_char(&self, char: char) {
        let idx = *self.browser_focus.borrow();
        let focus = FOCUS_BROWSER_ARRAY[idx as usize].clone();
        if matches!(focus, FocusBrowser::Port) {
            self.browser_port_state.borrow_mut().value.push(char)
        }
    }

    fn pop_ssh_input_char(&self) {
        let idx = *self.ssh_focus.borrow();
        let focus = FOCUS_SSH_ARRAY[idx as usize].clone();

        match focus {
            FocusSsh::User => self.ssh_user_state.borrow_mut().value.pop(),
            FocusSsh::Port => self.ssh_port_state.borrow_mut().value.pop(),
            FocusSsh::Identity => {
                self.ssh_identity_state.borrow_mut().value.pop()
            }
        };
    }

    fn pop_browser_input_char(&self) {
        let idx = *self.browser_focus.borrow();
        let focus = FOCUS_BROWSER_ARRAY[idx as usize].clone();

        match focus {
            FocusBrowser::Port => {
                self.browser_port_state.borrow_mut().value.pop()
            }
            _ => None,
        };
    }

    fn push_input_char(&self, char: char) {
        if *self.editing_browser.borrow() {
            self.push_browser_input_char(char);
        }

        if *self.editing_ssh.borrow() {
            self.push_ssh_input_char(char);
        }
    }

    fn pop_input_char(&self) {
        if *self.editing_browser.borrow() {
            self.pop_browser_input_char();
        }

        if *self.editing_ssh.borrow() {
            self.pop_ssh_input_char();
        }
    }

    fn update_focus_settings(&self) {
        let editing_ssh = *self.editing_ssh.borrow();
        let current_ssh = *self.ssh_focus.borrow();

        for (idx, focus) in FOCUS_SSH_ARRAY.iter().enumerate() {
            let editing = editing_ssh && idx == current_ssh as usize;
            match focus {
                FocusSsh::Identity => {
                    self.ssh_identity_state.borrow_mut().editing = editing;
                }
                FocusSsh::Port => {
                    self.ssh_port_state.borrow_mut().editing = editing;
                }
                FocusSsh::User => {
                    self.ssh_user_state.borrow_mut().editing = editing;
                }
            }
        }

        let editing_browser = *self.editing_browser.borrow();
        let current_browser = *self.browser_focus.borrow();

        for (idx, focus) in FOCUS_BROWSER_ARRAY.iter().enumerate() {
            let editing = editing_browser && idx == current_browser as usize;
            match focus {
                FocusBrowser::Port => {
                    self.browser_port_state.borrow_mut().editing = editing;
                }
                FocusBrowser::Select => {
                    self.browser_select_state.borrow_mut().editing = editing;
                }
            }
        }
    }

    fn reset_input_state(&self) {
        self.browser_select_state.borrow_mut().editing = false;
        self.browser_port_state.borrow_mut().editing = false;
        *self.browser_focus.borrow_mut() = 0;
        *self.editing_browser.borrow_mut() = false;
        self.ssh_user_state.borrow_mut().editing = false;
        self.ssh_port_state.borrow_mut().editing = false;
        self.ssh_identity_state.borrow_mut().editing = false;
        *self.ssh_focus.borrow_mut() = 0;
        *self.editing_ssh.borrow_mut() = false;
    }

    fn focus_next_ssh(&self) {
        let new_idx =
            (*self.ssh_focus.borrow() + 1) % FOCUS_SSH_ARRAY.len() as i8;
        *self.ssh_focus.borrow_mut() = new_idx;
        self.update_focus_settings();
    }

    fn focus_previous_ssh(&self) {
        let mut new_idx = *self.ssh_focus.borrow() - 1;
        if new_idx < 0 {
            new_idx = FOCUS_SSH_ARRAY.len() as i8 - 1
        }
        new_idx %= FOCUS_SSH_ARRAY.len() as i8;
        *self.ssh_focus.borrow_mut() = new_idx;
        self.update_focus_settings();
    }

    fn focus_next_browser(&self) {
        let new_idx = (*self.browser_focus.borrow() + 1)
            % FOCUS_BROWSER_ARRAY.len() as i8;
        *self.browser_focus.borrow_mut() = new_idx;
        self.update_focus_settings();
    }

    fn focus_previous_browser(&self) {
        let mut new_idx = *self.browser_focus.borrow() - 1;
        if new_idx < 0 {
            new_idx = FOCUS_BROWSER_ARRAY.len() as i8 - 1
        }
        new_idx %= FOCUS_BROWSER_ARRAY.len() as i8;
        *self.browser_focus.borrow_mut() = new_idx;
        self.update_focus_settings();
    }

    fn is_tracing(&self, state: &State) -> bool {
        if let Some(cmd) = state.cmd_in_progress.as_ref() {
            matches!(cmd, Command::TraceRoute(_))
        } else {
            false
        }
    }

    fn focus_next(&self) {
        if *self.editing_browser.borrow() {
            self.focus_next_browser();
        }

        if *self.editing_ssh.borrow() {
            self.focus_next_ssh();
        }
    }

    fn focus_previous(&self) {
        if *self.editing_browser.borrow() {
            self.focus_previous_browser();
        }

        if *self.editing_ssh.borrow() {
            self.focus_previous_ssh();
        }
    }
}

impl View for DeviceView {
    fn legend(&self, _state: &State) -> String {
        if *self.editing_browser.borrow()
            || *self.editing_ssh.borrow()
            || *self.confirm_config_removal.borrow()
        {
            "(esc) exit configuration | (enter) save configuration".into()
        } else {
            "(esc) back to devices | (c) configure | (s) SSH | (t) traceroute | (b) browse | (bckspc) reset device config".into()
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
    ) -> Result<()> {
        let [top_row, bottom_row] = Layout::vertical([
            Constraint::Length(12), // ssh config / sparkline
            Constraint::Min(0),     // device info / command output
        ])
        .areas(area);

        let [ssh_area, sparkline_area] = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .areas(top_row);

        let [info_area, cmd_area] = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .areas(bottom_row);

        self.render_device_ssh_config(ssh_area, buf, ctx);
        self.render_latency_sparkline(sparkline_area, buf, ctx);
        self.render_device_info(info_area, buf, ctx);
        self.render_cmd_output(cmd_area, buf, ctx);
        // important to render popovers last so they layer on top
        self.render_browser_config_popover(buf, ctx)?;
        self.render_config_removal_popover(buf, ctx)?;

        Ok(())
    }
}

impl EventHandler for DeviceView {
    fn process_event(
        &self,
        evt: &CrossTermEvent,
        ctx: &CustomEventContext,
    ) -> Result<bool> {
        match evt {
            CrossTermEvent::FocusGained => {}
            CrossTermEvent::FocusLost => {}
            // override scroll events from devices view
            CrossTermEvent::Mouse(_m) => {}
            CrossTermEvent::Paste(_s) => {}
            CrossTermEvent::Resize(_x, _y) => {}
            CrossTermEvent::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }
                            if *self.editing_browser.borrow()
                                || *self.editing_ssh.borrow()
                            {
                                self.reset_input_state();
                                return Ok(true);
                            } else if *self.confirm_config_removal.borrow() {
                                self.confirm_config_removal.replace(false);
                                return Ok(true);
                            } else {
                                ctx.dispatcher
                                    .dispatch(Action::ClearCommandOutput);
                                // allow this one to bubble up to next layer
                                return Ok(false);
                            }
                        }
                        KeyCode::Right => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }
                            if *self.editing_browser.borrow() {
                                if *self.browser_focus.borrow() == 0 {
                                    self.next_browser();
                                }
                                return Ok(true);
                            }
                            // allow tab change to bubble up
                            return Ok(false);
                        }
                        KeyCode::Left => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }
                            if *self.editing_browser.borrow() {
                                if *self.browser_focus.borrow() == 0 {
                                    self.next_browser();
                                }
                                return Ok(true);
                            }
                            // allow tab change to bubble up
                            return Ok(false);
                        }
                        KeyCode::Tab => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }
                            if *self.editing_browser.borrow()
                                || *self.editing_ssh.borrow()
                            {
                                self.focus_next();
                                return Ok(true);
                            }
                            // allow tab change to bubble up
                            return Ok(false);
                        }
                        KeyCode::BackTab => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }
                            if *self.editing_browser.borrow()
                                || *self.editing_ssh.borrow()
                            {
                                self.focus_previous();
                                return Ok(true);
                            }
                            // allow tab change to bubble up
                            return Ok(false);
                        }
                        KeyCode::Enter => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }

                            if *self.editing_browser.borrow() {
                                let port_str = self
                                    .browser_port_state
                                    .borrow()
                                    .value
                                    .clone();
                                if let Ok(port) = port_str.parse::<u16>() {
                                    ctx.ipc.send(MainMessage::ExecCommand(
                                        Command::Browse(BrowseArgs {
                                            device: self.device.clone(),
                                            port,
                                            use_lynx: self
                                                .browser_select_state
                                                .borrow()
                                                .value
                                                == "lynx",
                                        }),
                                    ))?;
                                    self.reset_input_state();
                                    return Ok(true);
                                }
                            }

                            if *self.editing_ssh.borrow() {
                                let mut device_config =
                                    self.device_config.clone();
                                device_config.ssh_user =
                                    self.ssh_user_state.borrow().value.clone();
                                let port = self
                                    .ssh_port_state
                                    .borrow()
                                    .value
                                    .clone()
                                    .parse::<u16>();
                                device_config.ssh_port = port.unwrap_or(22);
                                device_config.ssh_identity_file = self
                                    .ssh_identity_state
                                    .borrow()
                                    .value
                                    .clone();
                                ctx.dispatcher.dispatch(
                                    Action::UpdateDeviceConfig(device_config),
                                );
                                self.reset_input_state();
                                return Ok(true);
                            }

                            if *self.confirm_config_removal.borrow() {
                                ctx.dispatcher.dispatch(
                                    Action::RemoveDeviceConfig(
                                        self.device_config.id.clone(),
                                    ),
                                );

                                self.confirm_config_removal.replace(false);
                                return Ok(true);
                            }
                        }
                        KeyCode::Backspace => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }

                            if *self.editing_browser.borrow()
                                || *self.editing_ssh.borrow()
                            {
                                self.pop_input_char();
                                return Ok(true);
                            }

                            self.confirm_config_removal.replace(true);
                        }
                        KeyCode::Char(c) => {
                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }

                            if *self.editing_browser.borrow()
                                || *self.editing_ssh.borrow()
                            {
                                // handle value update for focused element
                                self.push_input_char(c);
                                return Ok(true);
                            }

                            if self.is_tracing(ctx.state) {
                                return Ok(true);
                            }

                            if c == 'c' {
                                // enter edit mode
                                *self.ssh_focus.borrow_mut() = 0;
                                self.ssh_user_state.borrow_mut().editing = true;
                                *self.editing_ssh.borrow_mut() = true;
                                return Ok(true);
                            }

                            if c == 's' && ctx.state.cmd_in_progress.is_none() {
                                let _ = ctx.ipc.send(MainMessage::ExecCommand(
                                    Command::Ssh(
                                        self.device.clone(),
                                        self.device_config.clone(),
                                    ),
                                ));
                                return Ok(true);
                            }

                            if c == 't' && !self.is_tracing(ctx.state) {
                                ctx.ipc.send(MainMessage::ExecCommand(
                                    Command::TraceRoute(self.device.clone()),
                                ))?;
                                return Ok(true);
                            }

                            if c == 'b' {
                                *self.browser_focus.borrow_mut() = 0;
                                let mut browser_state =
                                    self.browser_select_state.borrow_mut();
                                browser_state.editing = true;
                                browser_state.value = "default".to_string();
                                *self.editing_browser.borrow_mut() = true;
                                return Ok(true);
                            }

                            if c == 'f'
                                || c == 'd'
                                    && !*self.editing_browser.borrow()
                                    && !*self.editing_ssh.borrow()
                            {
                                // allow tab change to bubble up
                                return Ok(false);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // override all other key handlers from devices view
        Ok(true)
    }
}

#[cfg(test)]
#[path = "./device_tests.rs"]
mod tests;
