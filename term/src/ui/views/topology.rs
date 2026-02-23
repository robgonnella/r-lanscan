//! Topology view showing network devices grouped by normalized latency.

use color_eyre::eyre::Result;
use itertools::Itertools;
use r_lanlib::scanners::Device;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};
use std::cell::RefCell;

use crate::{store::state::State, ui::views::traits::CustomEventContext};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

/// Latency proximity bucket for grouping devices.
enum LatencyBucket {
    Direct,
    Near,
    Far,
}

impl LatencyBucket {
    fn label(&self) -> &'static str {
        match self {
            LatencyBucket::Direct => "Direct  +0–1ms (normalized)",
            LatencyBucket::Near => "Near    +1–10ms (normalized)",
            LatencyBucket::Far => "Far     +10ms+ (normalized)",
        }
    }

    fn from_normalized_ms(ms: u128) -> Self {
        if ms <= 1 {
            LatencyBucket::Direct
        } else if ms <= 10 {
            LatencyBucket::Near
        } else {
            LatencyBucket::Far
        }
    }
}

/// Truncates `s` to at most `max` characters, appending `…` if cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{}…", cut)
    }
}

/// Infers the OS class of a device from its observed response TTL.
fn os_class(ttl: u8) -> &'static str {
    match ttl {
        1..=64 => "Linux/macOS",
        65..=128 => "Windows",
        _ => "Network",
    }
}

/// Infers the initial TTL the remote host likely started with.
fn initial_ttl(observed: u8) -> u8 {
    if observed > 128 {
        255
    } else if observed > 64 {
        128
    } else {
        64
    }
}

/// Estimates the number of hops between the scanner and the device.
fn hop_count(observed: u8) -> u8 {
    initial_ttl(observed).saturating_sub(observed)
}

/// Returns the gateway-normalized latency for a device.
/// Falls back to raw latency if no gateway baseline is available.
fn normalized_latency(
    device: &Device,
    gateway_latency: Option<u128>,
) -> Option<u128> {
    let raw = device.latency_ms?;
    let baseline = gateway_latency.unwrap_or(0);
    Some(raw.saturating_sub(baseline))
}

/// View showing network devices grouped by proximity to the gateway.
#[derive(Default)]
pub struct TopologyView {
    scroll_offset: RefCell<u16>,
}

impl TopologyView {
    pub fn new() -> Self {
        Self {
            scroll_offset: RefCell::new(0),
        }
    }

    fn scroll_down(&self) {
        let next = self.scroll_offset.borrow().saturating_add(1);
        *self.scroll_offset.borrow_mut() = next;
    }

    fn scroll_up(&self) {
        let next = self.scroll_offset.borrow().saturating_sub(1);
        *self.scroll_offset.borrow_mut() = next;
    }

    fn build_lines<'a>(
        &self,
        devices: &'a [&'a Device],
        ctx: &CustomWidgetContext,
    ) -> Vec<Line<'a>> {
        let colors = &ctx.state.colors;

        let gateway = devices.iter().find(|d| d.is_gateway).copied();
        let gateway_latency = gateway.and_then(|g| g.latency_ms);

        let mut lines: Vec<Line<'_>> = Vec::new();

        // Gateway header
        match gateway {
            Some(gw) => {
                let latency_str = gw
                    .latency_ms
                    .map(|ms| format!("{}ms  (baseline)", ms))
                    .unwrap_or_else(|| "(no latency)  (baseline)".into());

                let vendor = if gw.vendor.is_empty() {
                    "[unknown vendor]"
                } else {
                    &gw.vendor
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        "[Gateway]  ",
                        Style::default()
                            .fg(colors.selected_row_fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<16}", gw.ip),
                        Style::default()
                            .fg(colors.selected_row_fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<26}", vendor),
                        Style::default().fg(colors.text),
                    ),
                    Span::styled(
                        latency_str,
                        Style::default().fg(colors.light_gray),
                    ),
                ]));
            }
            None => {
                lines.push(Line::from(Span::styled(
                    "[Gateway]  (not detected)",
                    Style::default().fg(colors.light_gray),
                )));
            }
        }

        lines.push(Line::from(Span::styled(
            "│",
            Style::default().fg(colors.border_color),
        )));

        // Non-gateway devices bucketed by normalized latency
        let non_gateway: Vec<&Device> =
            devices.iter().filter(|d| !d.is_gateway).copied().collect();

        if non_gateway.is_empty() {
            lines.push(Line::from(Span::styled(
                "    No devices discovered yet.",
                Style::default().fg(colors.light_gray),
            )));
            return lines;
        }

        let mut direct: Vec<&Device> = Vec::new();
        let mut near: Vec<&Device> = Vec::new();
        let mut far: Vec<&Device> = Vec::new();
        let mut unknown: Vec<&Device> = Vec::new();

        for device in &non_gateway {
            match normalized_latency(device, gateway_latency) {
                Some(ms) => match LatencyBucket::from_normalized_ms(ms) {
                    LatencyBucket::Direct => direct.push(device),
                    LatencyBucket::Near => near.push(device),
                    LatencyBucket::Far => far.push(device),
                },
                None => unknown.push(device),
            }
        }

        let buckets: Vec<(LatencyBucket, Vec<&Device>)> = vec![
            (LatencyBucket::Direct, direct),
            (LatencyBucket::Near, near),
            (LatencyBucket::Far, far),
        ];

        let non_empty: Vec<_> = buckets
            .iter()
            .filter(|(_, devs)| !devs.is_empty())
            .collect();

        for (bucket_idx, (bucket, devs)) in non_empty.iter().enumerate() {
            let is_last = bucket_idx == non_empty.len() - 1;
            let branch = if is_last { "└──" } else { "├──" };
            let child_prefix = if is_last { "    " } else { "│   " };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", branch),
                    Style::default().fg(colors.border_color),
                ),
                Span::styled(
                    format!("[{}]", bucket.label()),
                    Style::default()
                        .fg(colors.header_text)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            for device in devs.iter() {
                let norm = normalized_latency(device, gateway_latency);
                let latency_str = norm
                    .map(|ms| format!("+{}ms", ms))
                    .unwrap_or_else(|| "—".into());

                let hostname = if device.hostname.is_empty() {
                    "[unknown]".to_string()
                } else {
                    device.hostname.clone()
                };

                let vendor = if device.vendor.is_empty() {
                    "[unknown vendor]".to_string()
                } else {
                    device.vendor.clone()
                };

                let host_tag =
                    if device.is_current_host { " [YOU]" } else { "" };

                let os_str = device.response_ttl.map(os_class).unwrap_or("—");

                let hops_str = device
                    .response_ttl
                    .map(|ttl| format!("{} hops", hop_count(ttl)))
                    .unwrap_or_else(|| "—".into());

                // Truncate to column width so padding always produces
                // exact-width columns and values never push neighbours.
                let hostname = truncate(&hostname, 24);
                let vendor = truncate(&vendor, 24);

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}  ", child_prefix),
                        Style::default().fg(colors.border_color),
                    ),
                    Span::styled(
                        format!("{:<16}", device.ip),
                        Style::default().fg(colors.text),
                    ),
                    Span::styled(
                        format!("{:<26}", hostname),
                        Style::default().fg(colors.text),
                    ),
                    Span::styled(
                        format!("{:<26}", vendor),
                        Style::default().fg(colors.light_gray),
                    ),
                    Span::styled(
                        format!("{:<13}", os_str),
                        Style::default().fg(colors.light_gray),
                    ),
                    Span::styled(
                        format!("{:<9}", hops_str),
                        Style::default().fg(colors.light_gray),
                    ),
                    Span::styled(
                        format!("{}{}", latency_str, host_tag),
                        Style::default().fg(colors.selected_row_fg),
                    ),
                ]));
            }
        }

        lines
    }

    fn render_topology(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let devices = ctx.state.device_list();
        let sorted: Vec<&Device> =
            devices.into_iter().sorted_by_key(|d| d.ip).collect();

        let lines = self.build_lines(&sorted, ctx);
        let total = lines.len() as u16;
        let visible = area.height;

        // Clamp the offset so we never scroll past the last line.
        let max_offset = total.saturating_sub(visible);
        let offset = (*self.scroll_offset.borrow()).min(max_offset);
        *self.scroll_offset.borrow_mut() = offset;

        let text = Text::from(lines);

        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((offset, 0))
            .render(area, buf);
    }
}

impl View for TopologyView {
    fn legend(&self, _state: &State) -> String {
        "(j/k) scroll".into()
    }
}

impl CustomWidgetRef for TopologyView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let [_, topology_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)])
                .areas(area);

        self.render_topology(topology_area, buf, ctx);
        Ok(())
    }
}

impl EventHandler for TopologyView {
    fn process_event(
        &self,
        evt: &Event,
        ctx: &CustomEventContext,
    ) -> Result<bool> {
        let mut handled = false;

        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    if !ctx.state.device_map.is_empty() {
                        self.scroll_down();
                    }
                    handled = true;
                }
                MouseEventKind::ScrollUp => {
                    if !ctx.state.device_map.is_empty() {
                        self.scroll_up();
                    }
                    handled = true;
                }
                _ => {}
            },
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !ctx.state.device_map.is_empty() {
                                self.scroll_down();
                            }
                            handled = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if !ctx.state.device_map.is_empty() {
                                self.scroll_up();
                            }
                            handled = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(handled)
    }
}

#[cfg(test)]
#[path = "./topology_tests.rs"]
mod tests;
