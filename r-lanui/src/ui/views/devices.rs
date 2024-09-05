use std::sync::Arc;

use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::ui::store::{action::Action, dispatcher::Dispatcher, store::Colors, types::ViewName};

use super::View;

const ITEM_HEIGHT: usize = 4;

const ELLIPSIS: &str = "…";

const COLUMN_WIDTH: u16 = 50;

const INFO_TEXT: &str =
    "(Esc) quit | (↑) move up | (↓) move down | (Enter) view selected device | (c) manage config";

pub struct DevicesView {
    pub id: ViewName,
    table_state: TableState,
    dispatcher: Arc<Dispatcher>,
    scroll_state: ScrollbarState,
}

impl DevicesView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let state = dispatcher.get_state();

        let mut height = ITEM_HEIGHT;

        if state.devices.len() > 0 {
            height = (state.devices.len() - 1) * ITEM_HEIGHT;
        }

        Self {
            id: ViewName::Devices,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(height),
            dispatcher,
        }
    }

    fn next(&mut self) {
        let data = self.dispatcher.get_state().devices;

        let i = match self.table_state.selected() {
            Some(i) => (i + 1) % data.len(),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
        self.dispatcher.dispatch(Action::UpdateSelectedDevice(&i));
    }

    fn previous(&mut self) {
        let data = self.dispatcher.get_state().devices;

        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    data.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
        self.dispatcher.dispatch(Action::UpdateSelectedDevice(&i));
    }

    fn render_table(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let header_style = Style::default()
            .fg(colors.header_fg)
            .bg(colors.header_bg)
            .add_modifier(Modifier::BOLD);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(colors.selected_style_fg);

        let header: Row<'_> = ["Hostname", "IP", "MAC", "Vendor", "Ports"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let items = self.dispatcher.get_state().devices;

        let rows = items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => colors.normal_row_color,
                _ => colors.alt_row_color,
            };
            let col_width = area.width / 5;
            let item = ref_array_from_device(data, col_width as usize);
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(colors.row_fg).bg(color))
                .height(ITEM_HEIGHT as u16)
        });

        let spacer = "  ";
        let t = Table::new(
            rows,
            [
                Constraint::Max(COLUMN_WIDTH),
                Constraint::Max(COLUMN_WIDTH),
                Constraint::Max(COLUMN_WIDTH),
                Constraint::Max(COLUMN_WIDTH),
                Constraint::Max(COLUMN_WIDTH),
            ],
        )
        .header(header)
        .highlight_style(selected_style)
        .highlight_symbol(Text::from(vec![
            spacer.into(),
            spacer.into(),
            spacer.into(),
            spacer.into(),
        ]))
        .bg(colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);
        f.render_stateful_widget(t, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, f: &mut Frame, area: Rect) {
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_footer(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let info_footer = Paragraph::new(Line::from(INFO_TEXT))
            .style(Style::new().fg(colors.row_fg).bg(colors.buffer_bg))
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(colors.footer_border_color)),
            );
        f.render_widget(info_footer, area);
    }
}

impl View for DevicesView {
    fn render(&mut self, f: &mut Frame) {
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(f.area());
        let colors = self.dispatcher.get_state().colors;

        self.render_table(f, rects[0], &colors);

        self.render_scrollbar(f, rects[0]);

        self.render_footer(f, rects[1], &colors);
    }

    fn process_key_event(&mut self, key: KeyEvent) -> bool {
        let mut handled = false;
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.next();
                    handled = true;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.previous();
                    handled = true
                }
                KeyCode::Char('c') => {
                    self.dispatcher
                        .dispatch(Action::UpdateView(&ViewName::Config));
                    handled = true;
                }
                KeyCode::Enter => {
                    self.dispatcher
                        .dispatch(Action::UpdateView(&ViewName::Device));
                    handled = true;
                }
                _ => {}
            }
        }

        handled
    }
}

fn ref_array_from_device(device: &DeviceWithPorts, col_width: usize) -> [String; 5] {
    let hostname_len: usize = device.hostname.width();
    let ip_len = device.ip.width();
    let mac_len = device.mac.width();
    let vendor_len = device.vendor.width();
    let mut ports = device
        .open_ports
        .iter()
        .sorted_by_key(|d| d.id)
        .map(|d| d.id.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let ports_len = ports.width();

    let mut hostname = device.hostname.to_owned();
    if hostname_len >= col_width {
        hostname.truncate(col_width - 10);
        hostname.push_str(ELLIPSIS);
    }
    let mut ip = device.ip.to_owned();
    if ip_len >= col_width {
        ip.truncate(col_width - 10);
        ip.push_str(ELLIPSIS);
    }
    let mut mac = device.mac.to_owned();
    if mac_len >= col_width {
        mac.truncate(col_width - 10);
        mac.push_str(ELLIPSIS);
    }
    let mut vendor = device.vendor.to_owned();
    if vendor_len >= col_width {
        vendor.truncate(col_width - 10);
        vendor.push_str(ELLIPSIS);
    }

    if ports_len >= col_width {
        ports.truncate(col_width - 10);
        ports.push_str(ELLIPSIS);
    }
    [hostname, ip, mac, vendor, ports]
}
