use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::{
    config::{Config, DeviceConfig},
    ui::store::store::Colors,
};

pub struct DeviceInfo<'c> {
    device: DeviceWithPorts,
    _device_config: DeviceConfig,
    config: Config,
    colors: &'c Colors,
}

impl<'c> DeviceInfo<'c> {
    pub fn new(
        device: DeviceWithPorts,
        device_config: DeviceConfig,
        config: Config,
        colors: &'c Colors,
    ) -> Self {
        Self {
            device,
            _device_config: device_config,
            config,
            colors,
        }
    }
}

impl<'c> Widget for DeviceInfo<'c> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let spacer1 = Line::from("");
        let spacer2 = Line::from("");
        let spacer3 = Line::from("");

        let hostname = Line::from(format!("  Hostname: {0}", self.device.hostname));
        let ip = Line::from(format!("  IP: {0}", self.device.ip));
        let mac = Line::from(format!("  MAC: {0}", self.device.mac));
        let vendor = Line::from(format!("  Vendor: {0}", self.device.vendor));

        let scanned_ports = Line::from(format!(
            "  Scanned Ports: {0}",
            self.config.ports.join(", ")
        ));

        let open_ports = Line::from(format!(
            "  Open Ports: {0}",
            self.device
                .open_ports
                .iter()
                .sorted_by_key(|p| p.id)
                .map(|p| p.id.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ));

        let info = Paragraph::new(vec![
            spacer1,
            hostname,
            ip,
            mac,
            vendor,
            spacer2,
            scanned_ports,
            spacer3,
            open_ports,
        ])
        .style(
            Style::new()
                .fg(self.colors.row_fg)
                .bg(self.colors.buffer_bg),
        )
        .wrap(Wrap { trim: true })
        .left_aligned();

        info.render(area, buf)
    }
}
