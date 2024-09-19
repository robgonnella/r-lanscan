use itertools::Itertools;
use r_lanlib::scanners::DeviceWithPorts;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{
        Block, BorderType, Paragraph, ScrollbarState, StatefulWidget, TableState, Widget, WidgetRef,
    },
};
use std::{cell::RefCell, sync::Arc};

use crate::ui::{
    components::{
        footer::InfoFooter,
        scrollbar::ScrollBar,
        table::{self, Table},
    },
    store::{action::Action, dispatcher::Dispatcher, store::Colors, types::ViewName},
};

use super::{EventHandler, View};

const HEADERS: [&str; 5] = ["Hostname", "IP", "MAC", "Vendor", "Ports"];

const INFO_TEXT: &str =
    "(Esc) quit | (↑) move up | (↓) move down | (Enter) view selected device | (c) manage config";

pub struct DevicesView {
    dispatcher: Arc<Dispatcher>,
    table_state: RefCell<TableState>,
    scroll_state: RefCell<ScrollbarState>,
}

impl DevicesView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let state = dispatcher.get_state();

        let mut height = table::ITEM_HEIGHT;

        if state.devices.len() > 0 {
            height = (state.devices.len() - 1) * table::ITEM_HEIGHT;
        }

        Self {
            table_state: RefCell::new(TableState::default().with_selected(0)),
            scroll_state: RefCell::new(ScrollbarState::new(height)),
            dispatcher,
        }
    }

    fn next(&mut self) {
        let devices = self.dispatcher.get_state().devices;

        let i = match self.table_state.borrow().selected() {
            Some(i) => (i + 1) % devices.len(),
            None => 0,
        };

        self.table_state.borrow_mut().select(Some(i));

        let new_scroll_state = self.scroll_state.borrow().position(i * table::ITEM_HEIGHT);

        self.scroll_state = RefCell::new(new_scroll_state);

        self.set_store_selected(devices, i);
    }

    fn previous(&mut self) {
        let devices = self.dispatcher.get_state().devices;
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    devices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.table_state.borrow_mut().select(Some(i));

        let new_scroll_state = self.scroll_state.borrow().position(i * table::ITEM_HEIGHT);

        self.scroll_state = RefCell::new(new_scroll_state);

        self.set_store_selected(devices, i);
    }

    fn set_store_selected(&self, devices: Vec<DeviceWithPorts>, i: usize) {
        if devices.len() > 0 {
            let mac = devices[i].mac.clone();
            self.dispatcher.dispatch(Action::UpdateSelectedDevice(&mac));
        }
    }

    fn render_info(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, colors: &Colors) {
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
        info.render(area, buf);
    }

    fn render_table(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, colors: &Colors) {
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
        let table = Table::new(items, headers, colors);
        let mut table_state = self.table_state.borrow_mut();
        table.render(area, buf, &mut table_state);
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let scrollbar = ScrollBar::new();
        let mut scroll_state = self.scroll_state.borrow_mut();
        scrollbar.render(area, buf, &mut scroll_state);
    }

    fn render_footer(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, colors: &Colors) {
        let footer = InfoFooter::new(INFO_TEXT.to_string(), colors);
        footer.render(area, buf);
    }
}

impl View for DevicesView {}

impl WidgetRef for DevicesView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        if let Some(selected_idx) = self.table_state.borrow().selected() {
            self.set_store_selected(state.devices, selected_idx);
        }

        let rects = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

        self.render_info(rects[0], buf, &state.colors);
        self.render_table(rects[1], buf, &state.colors);
        self.render_scrollbar(rects[1], buf);
        self.render_footer(rects[2], buf, &state.colors);
    }
}

impl EventHandler for DevicesView {
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
                            if let Some(_selected) = self.table_state.borrow().selected() {
                                self.dispatcher
                                    .dispatch(Action::UpdateView(&ViewName::Device));
                                handled = true;
                            }
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
