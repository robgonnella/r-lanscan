use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
};
use std::{cell::RefCell, sync::Arc};

use crate::ui::{
    components::table::{self, Table},
    store::{
        action::Action,
        state::{State, ViewID},
        store::Store,
    },
};

use super::traits::{CustomWidgetRef, EventHandler, View};

pub struct DevicesView {
    store: Arc<Store>,
    table: RefCell<Table>,
}

impl DevicesView {
    pub fn new(store: Arc<Store>) -> Self {
        let state = store.get_state();

        let items = state
            .devices
            .iter()
            .map(|d| {
                vec![
                    d.ip.clone(),
                    d.hostname.clone(),
                    d.vendor.clone(),
                    d.mac.clone(),
                    d.open_ports
                        .iter()
                        .sorted_by_key(|p| p.id)
                        .map(|p| p.id.to_string())
                        .join(", "),
                ]
            })
            .collect_vec();

        let mut height = table::DEFAULT_ITEM_HEIGHT;

        if state.devices.len() > 0 {
            height = (state.devices.len() - 1) * table::DEFAULT_ITEM_HEIGHT;
        }

        Self {
            store,
            table: RefCell::new(Table::new(
                items,
                Some(vec![
                    "IP".to_string(),
                    "HOSTNAME".to_string(),
                    "VENDOR".to_string(),
                    "MAC".to_string(),
                    "OPEN PORTS".to_string(),
                ]),
                vec![20, 20, 20, 17, 30],
                height,
            )),
        }
    }

    fn next(&self) {
        self.table.borrow_mut().next();
    }

    fn previous(&self) {
        self.table.borrow_mut().previous();
    }

    fn set_store_selected(&self, i: usize, state: &State) {
        if state.devices.len() > 0 && i < state.devices.len() {
            let mac = state.devices[i].mac.clone();
            self.store.dispatch(Action::UpdateSelectedDevice(mac));
        }
    }

    fn handle_device_selection(&self, state: &State) {
        if state.selected_device.is_some() {
            self.store.dispatch(Action::UpdateView(ViewID::Device));
        }
    }

    fn render_table(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
        total_area: Rect,
    ) {
        let items = state
            .devices
            .iter()
            .map(|d| {
                vec![
                    if d.is_current_host {
                        format!("{} [YOU]", d.ip.clone())
                    } else {
                        d.ip.clone()
                    },
                    d.hostname.clone(),
                    d.vendor.clone(),
                    d.mac.clone(),
                    d.open_ports
                        .iter()
                        .sorted_by_key(|p| p.id)
                        .map(|p| p.id.to_string())
                        .join(", "),
                ]
            })
            .collect_vec();
        let selected = self.table.borrow_mut().update_items(items);
        if let Some(selected) = selected {
            self.set_store_selected(selected, state);
        }
        self.table.borrow().render_ref(area, buf, state, total_area);
    }
}

impl View for DevicesView {
    fn id(&self) -> ViewID {
        ViewID::Devices
    }

    fn legend(&self, _state: &State) -> &str {
        "(enter) view device details"
    }
}

impl CustomWidgetRef for DevicesView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
        total_area: Rect,
    ) {
        if let Some(selected_idx) = self.table.borrow().selected() {
            self.set_store_selected(selected_idx, state);
        }

        let view_rects = Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(area);

        self.render_table(view_rects[1], buf, &state, total_area);
    }
}

impl EventHandler for DevicesView {
    fn process_event(&self, evt: &Event, state: &State) -> bool {
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
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            self.next();
                            handled = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.previous();
                            handled = true;
                        }
                        KeyCode::Enter => {
                            self.handle_device_selection(state);
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
