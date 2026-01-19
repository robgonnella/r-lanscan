use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
};
use std::{cell::RefCell, sync::Arc};

use crate::ui::{
    components::table::{DEFAULT_ITEM_HEIGHT, Table},
    store::{
        Dispatcher,
        action::Action,
        state::{State, ViewID},
    },
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

pub struct DevicesView {
    dispatcher: Arc<dyn Dispatcher>,
    table: RefCell<Table>,
}

impl DevicesView {
    pub fn new(dispatcher: Arc<dyn Dispatcher>) -> Self {
        Self {
            dispatcher,
            table: RefCell::new(Table::new(
                Vec::new(),
                Some(vec![
                    "IP".to_string(),
                    "HOSTNAME".to_string(),
                    "VENDOR".to_string(),
                    "MAC".to_string(),
                    "OPEN PORTS".to_string(),
                ]),
                vec![20, 20, 20, 17, 30],
                DEFAULT_ITEM_HEIGHT,
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
        if !state.devices.is_empty() && i < state.devices.len() {
            let ip = state.devices[i].ip.clone();
            self.dispatcher.dispatch(Action::UpdateSelectedDevice(ip));
        }
    }

    fn handle_device_selection(&self, state: &State) {
        if state.selected_device.is_some() {
            self.dispatcher.dispatch(Action::UpdateView(ViewID::Device));
        }
    }

    fn render_table(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let items = ctx
            .state
            .devices
            .iter()
            .map(|d| {
                vec![
                    if d.is_current_host {
                        format!("{} [YOU]", d.ip)
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
            self.set_store_selected(selected, ctx.state);
        }
        self.table.borrow().render_ref(area, buf, ctx);
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
        ctx: &CustomWidgetContext,
    ) {
        if let Some(selected_idx) = self.table.borrow().selected() {
            self.set_store_selected(selected_idx, ctx.state);
        }

        let view_rects = Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(area);

        self.render_table(view_rects[1], buf, ctx);
    }
}

impl EventHandler for DevicesView {
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if ctx.state.render_view_select {
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
                            self.handle_device_selection(ctx.state);
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

#[cfg(test)]
#[path = "./devices_tests.rs"]
mod tests;
