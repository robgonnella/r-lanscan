use crate::{
    config::DeviceConfig,
    ui::{
        components::{device_info::DeviceInfo, header::Header},
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
    widgets::WidgetRef,
};
use std::sync::Arc;

use super::{CustomWidget, EventHandler, View};

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
        device: Option<DeviceWithPorts>,
        device_config: &DeviceConfig,
        state: &State,
    ) {
        if let Some(device) = device {
            let device_info = DeviceInfo::new(device.clone(), device_config.clone());

            device_info.render(area, buf, state);
        }
    }
}

impl View for DeviceView {
    fn id(&self) -> ViewID {
        ViewID::Device
    }
    fn legend(&self) -> &str {
        "(c) configure | (t) trace | (s) SSH"
    }
}

impl WidgetRef for DeviceView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        let view_rects = Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(area);

        let label_rects = Layout::horizontal([Constraint::Length(20)]).split(view_rects[0]);

        let header = Header::new(String::from("Device Info"));

        header.render(label_rects[0], buf, &state);

        if let Some(selected) = state.selected_device.clone() {
            if let Some(device) = state.device_map.get(&selected) {
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

                self.render_device_info(
                    view_rects[1],
                    buf,
                    Some(device.clone()),
                    &device_config,
                    &state,
                );
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
                    handled = true;
                    self.dispatcher
                        .dispatch(Action::UpdateView(ViewID::Devices));
                }
                _ => {}
            },
        }

        handled
    }
}
