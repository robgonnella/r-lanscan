use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::ui::{store::state::State, views::CustomWidget};

pub struct DeviceInfo {
    device: DeviceWithPorts,
}

impl DeviceInfo {
    pub fn new(device: DeviceWithPorts) -> Self {
        Self { device }
    }
}

impl CustomWidget for DeviceInfo {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State)
    where
        Self: Sized,
    {
        let spacer1 = "";

        let hostname = format!("Hostname: {0}", self.device.hostname);
        let ip = format!("IP: {0}", self.device.ip);
        let mac = format!("MAC: {0}", self.device.mac);
        let vendor = format!("Vendor: {0}", self.device.vendor);
        let open_ports = format!(
            "Open Ports: {0}",
            self.device
                .open_ports
                .iter()
                .sorted_by_key(|p| p.id)
                .map(|p| p.id.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );

        let info = Paragraph::new(vec![
            Line::from(spacer1),
            Line::from(hostname),
            Line::from(ip),
            Line::from(mac),
            Line::from(vendor),
            Line::from(open_ports),
        ])
        .style(
            Style::new()
                .fg(state.colors.row_fg)
                .bg(state.colors.buffer_bg),
        )
        .wrap(Wrap { trim: true })
        .left_aligned();

        info.render(area, buf);
    }
}
