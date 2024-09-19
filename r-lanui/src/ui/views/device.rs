use crate::{
    config::{Config, DeviceConfig},
    ui::{
        components::{device_info::DeviceInfo, footer::InfoFooter, header::Header},
        store::{action::Action, dispatcher::Dispatcher, store::Colors, types::ViewName},
    },
};
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::{Widget, WidgetRef},
};
use std::{env, sync::Arc};

use super::{EventHandler, View};

const INFO_TEXT: &str = "(Esc) back to main view";

pub struct DeviceView {
    dispatcher: Arc<Dispatcher>,
}

impl DeviceView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        Self { dispatcher }
    }

    fn render_device_info(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        device: &DeviceWithPorts,
        device_config: &DeviceConfig,
        config: &Config,
        colors: &Colors,
    ) {
        let section_rects = Layout::horizontal([Constraint::Percentage(50)]).split(area);

        let info_rects =
            Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(section_rects[0]);

        let header = Header::new("  Device Info".to_string(), colors);

        let device_info = DeviceInfo::new(
            device.clone(),
            device_config.clone(),
            config.clone(),
            colors,
        );

        header.render(info_rects[0], buf);
        device_info.render(info_rects[1], buf);
    }

    // fn render_scanned_ports(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {}

    // fn render_open_ports(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {}

    fn render_footer(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, colors: &Colors) {
        let footer = InfoFooter::new(INFO_TEXT.to_string(), colors);
        footer.render(area, buf);
    }
}

impl View for DeviceView {}

impl WidgetRef for DeviceView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        let device = state
            .device_map
            .get(&state.selected_device.unwrap())
            .unwrap();

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
                ssh_identity_file: "~/.ssh/id_rsa".to_string(),
                ssh_port: 22,
                ssh_user: env::var("USER").unwrap(),
            }
        }

        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);

        self.render_device_info(
            rects[0],
            buf,
            device,
            &device_config,
            &state.config,
            &state.colors,
        );
        // self.render_scanned_ports(f, rects[0], &colors);
        // self.render_open_ports(f, rects[0], &colors);
        self.render_footer(rects[1], buf, &state.colors);
    }
}

impl EventHandler for DeviceView {
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
