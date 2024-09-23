use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::WidgetRef,
};
use std::{cell::RefCell, sync::Arc};

use crate::ui::{
    components::{
        header::Header,
        table::{self, Table},
    },
    store::{
        action::Action,
        dispatcher::Dispatcher,
        state::{State, ViewID},
    },
};

use super::{CustomWidget, CustomWidgetRef, EventHandler, View};

pub struct DevicesView {
    dispatcher: Arc<Dispatcher>,
    table: RefCell<Table>,
}

impl DevicesView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let state = dispatcher.get_state();

        let items = state
            .devices
            .iter()
            .map(|d| vec![d.ip.clone()])
            .collect_vec();

        let mut height = table::DEFAULT_ITEM_HEIGHT;

        if state.devices.len() > 0 {
            height = (state.devices.len() - 1) * table::DEFAULT_ITEM_HEIGHT;
        }

        Self {
            dispatcher,
            table: RefCell::new(Table::new(items, None, 1, height)),
        }
    }

    fn next(&mut self) {
        let i = self.table.borrow_mut().next();
        self.set_store_selected(i);
    }

    fn previous(&mut self) {
        let i = self.table.borrow_mut().previous();
        self.set_store_selected(i);
    }

    fn set_store_selected(&self, i: usize) {
        let devices = self.dispatcher.get_state().devices;

        if devices.len() > 0 && i < devices.len() {
            let mac = devices[i].mac.clone();
            self.dispatcher.dispatch(Action::UpdateSelectedDevice(&mac));
        }
    }

    fn render_label(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let header = Header::new(String::from("Detected Devices"));
        header.render(area, buf, state);
    }

    fn render_table(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let items = state
            .devices
            .iter()
            .map(|d| vec![d.ip.clone()])
            .collect_vec();
        let selected = self.table.borrow_mut().update_items(items);
        if let Some(selected) = selected {
            self.set_store_selected(selected);
        }
        self.table.borrow().render_ref(area, buf, state);
    }
}

impl View for DevicesView {
    fn id(&self) -> ViewID {
        ViewID::Devices
    }
}

impl WidgetRef for DevicesView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        if let Some(selected_idx) = self.table.borrow().selected() {
            self.set_store_selected(selected_idx);
        }

        let view_rects = Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(area);

        let label_rects = Layout::horizontal([Constraint::Length(20)]).split(view_rects[0]);

        self.render_label(label_rects[0], buf, &state);
        self.render_table(view_rects[1], buf, &state);
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
                        _ => {}
                    }
                }
            }
        }

        handled
    }
}
