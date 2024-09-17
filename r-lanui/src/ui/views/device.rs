use crate::ui::{
    components::{field::Field, footer::InfoFooter},
    store::{action::Action, dispatcher::Dispatcher, store::Colors, types::ViewName},
};
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    Frame,
};
use std::sync::Arc;

use super::View;
use crate::ui::components::Component;

const INFO_TEXT: &str = "(Esc) back to main view";

pub struct DeviceView {
    dispatcher: Arc<Dispatcher>,
}

impl DeviceView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        Self { dispatcher }
    }

    fn render_device_info(
        &mut self,
        f: &mut Frame,
        area: Rect,
        colors: &Colors,
        device: &DeviceWithPorts,
    ) {
        let rects = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(area);

        let mut host_field = Field::new("Hostname".to_string(), device.hostname.clone());
        let mut ip_field = Field::new("IP".to_string(), device.ip.clone());
        let mut mac_field = Field::new("MAC".to_string(), device.mac.clone());
        let mut vendor_field = Field::new("Vendor".to_string(), device.vendor.clone());

        host_field.render(f, rects[0], colors);
        ip_field.render(f, rects[1], colors);
        mac_field.render(f, rects[2], colors);
        vendor_field.render(f, rects[3], colors);
    }

    // fn render_scanned_ports(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {}

    // fn render_open_ports(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {}

    fn render_footer(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let mut footer = InfoFooter::new(INFO_TEXT.to_string());
        footer.render(f, area, colors);
    }
}

impl View for DeviceView {
    fn render(&mut self, f: &mut Frame) {
        let state = self.dispatcher.get_state();

        let device = state
            .device_map
            .get(&state.selected_device.unwrap())
            .unwrap();

        // let mut device_config: DeviceConfig;

        // if state.config.device_configs.contains_key(&device.ip) {
        //     device_config = state.config.device_configs.get(&device.ip).unwrap().clone();
        // } else if state.config.device_configs.contains_key(&device.mac) {
        //     device_config = state
        //         .config
        //         .device_configs
        //         .get(&device.mac)
        //         .unwrap()
        //         .clone();
        // } else {
        //     device_config = DeviceConfig {
        //         id: device.mac.clone(),
        //         ssh_identity_file: "~/.ssh/id_rsa".to_string(),
        //         ssh_port: 22,
        //         ssh_user: env::var("USER").unwrap(),
        //     }
        // }

        let colors = self.dispatcher.get_state().colors;
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(f.area());

        self.render_device_info(f, rects[0], &colors, device);
        // self.render_scanned_ports(f, rects[0], &colors);
        // self.render_open_ports(f, rects[0], &colors);
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
                        _ => {}
                    }
                }
            }
        }

        handled
    }
}
