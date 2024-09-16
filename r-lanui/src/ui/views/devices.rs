use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Paragraph, ScrollbarState, TableState},
    Frame,
};
use std::sync::Arc;

use crate::ui::{
    components::{
        footer::InfoFooter,
        scrollbar::ScrollBar,
        table::{self, Table},
        Component,
    },
    store::{action::Action, dispatcher::Dispatcher, store::Colors, types::ViewName},
};

use super::View;

const HEADERS: [&str; 5] = ["Hostname", "IP", "MAC", "Vendor", "Ports"];

const INFO_TEXT: &str =
    "(Esc) quit | (↑) move up | (↓) move down | (Enter) view selected device | (c) manage config";

pub struct DevicesView {
    dispatcher: Arc<Dispatcher>,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl DevicesView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let state = dispatcher.get_state();

        let mut height = table::ITEM_HEIGHT;

        if state.devices.len() > 0 {
            height = (state.devices.len() - 1) * table::ITEM_HEIGHT;
        }

        Self {
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
        self.scroll_state = self.scroll_state.position(i * table::ITEM_HEIGHT);
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
        self.scroll_state = self.scroll_state.position(i * table::ITEM_HEIGHT);
        self.dispatcher.dispatch(Action::UpdateSelectedDevice(&i));
    }

    fn render_info(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let state = self.dispatcher.get_state();
        let cidr = state.config.cidr;
        let ports = state.config.ports.join(", ");
        let content = format!("Scanning: CIDR: {cidr}, Ports: {ports}");
        let info = Paragraph::new(Line::from(content))
            .style(Style::new().fg(colors.row_fg).bg(colors.buffer_bg))
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(colors.footer_border_color)),
            );
        f.render_widget(info, area);
    }

    fn render_table(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let state = self.dispatcher.get_state();
        let items = state
            .devices
            .iter()
            .map(table_row_from_device)
            .collect_vec();
        let headers = HEADERS
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>();
        let mut table = Table::new(items, headers, &mut self.table_state);
        table.render(f, area, colors);
    }

    fn render_scrollbar(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let mut scrollbar = ScrollBar::new(&mut self.scroll_state);
        scrollbar.render(f, area, colors);
    }

    fn render_footer(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let mut footer = InfoFooter::new(INFO_TEXT.to_string());
        footer.render(f, area, colors);
    }
}

impl View for DevicesView {
    fn render(&mut self, f: &mut Frame) {
        let rects = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());
        let colors = self.dispatcher.get_state().colors;
        self.render_info(f, rects[0], &colors);
        self.render_table(f, rects[1], &colors);
        self.render_scrollbar(f, rects[1], &colors);
        self.render_footer(f, rects[2], &colors);
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
            }
        }

        handled
    }
}

fn table_row_from_device(device: &DeviceWithPorts) -> Vec<String> {
    let ports = device
        .open_ports
        .iter()
        .sorted_by_key(|d| d.id)
        .map(|d| d.id.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    vec![
        device.hostname.clone(),
        device.ip.clone(),
        device.mac.clone(),
        device.vendor.clone(),
        ports,
    ]
}
