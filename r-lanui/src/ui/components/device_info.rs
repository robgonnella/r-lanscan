use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::{
    config::DeviceConfig,
    ui::{store::state::State, views::CustomWidget},
};

pub struct DeviceInfo {
    device: DeviceWithPorts,
    _device_config: DeviceConfig,
}

impl DeviceInfo {
    pub fn new(device: DeviceWithPorts, device_config: DeviceConfig) -> Self {
        Self {
            device,
            _device_config: device_config,
        }
    }
}

impl CustomWidget for DeviceInfo {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State)
    where
        Self: Sized,
    {
        let spacer1 = Line::from("");

        let hostname = Line::from(format!("Hostname: {0}", self.device.hostname));
        let ip = Line::from(format!("IP: {0}", self.device.ip));
        let mac = Line::from(format!("MAC: {0}", self.device.mac));
        let vendor = Line::from(format!("Vendor: {0}", self.device.vendor));
        let open_ports = Line::from(format!(
            "Open Ports: {0}",
            self.device
                .open_ports
                .iter()
                .sorted_by_key(|p| p.id)
                .map(|p| p.id.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ));

        let info = Paragraph::new(vec![spacer1, hostname, ip, mac, vendor, open_ports])
            .style(
                Style::new()
                    .fg(state.colors.row_fg)
                    .bg(state.colors.buffer_bg),
            )
            .wrap(Wrap { trim: true })
            .left_aligned();

        info.render(area, buf)
    }
}
