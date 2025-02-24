use std::{cell::RefCell, sync::Arc};

use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    widgets::WidgetRef,
};

use crate::ui::{
    components::table::{self, Table},
    store::{
        action::Action,
        dispatcher::Dispatcher,
        state::{State, ViewID},
    },
};

use super::{CustomWidgetRef, EventHandler, View};

pub struct ViewSelect {
    dispatcher: Arc<Dispatcher>,
    view_ids: Vec<ViewID>,
    table: RefCell<Table>,
}

impl ViewSelect {
    pub fn new(view_ids: Vec<ViewID>, padding: usize, dispatcher: Arc<Dispatcher>) -> Self {
        let mut spacer = String::from("");

        if padding > 0 {
            for _ in 0..padding {
                spacer += " ";
            }
        }

        let table_items = view_ids
            .clone()
            .iter()
            .map(|v| vec![format!("{}{}", spacer, v.to_string())])
            .collect_vec();

        let mut table_select = Table::new(
            table_items,
            None,
            vec![15; view_ids.len()],
            table::DEFAULT_ITEM_HEIGHT,
        );

        table_select.next();

        Self {
            dispatcher,
            view_ids,
            table: RefCell::new(table_select),
        }
    }

    fn next(&mut self) {
        self.table.borrow_mut().next();
    }

    fn previous(&mut self) {
        self.table.borrow_mut().previous();
    }

    fn handle_selected(&self) {
        let i = self.table.borrow().selected();
        if let Some(selected) = i {
            let id = self.view_ids[selected].clone();
            self.dispatcher.dispatch(Action::UpdateView(id));
            self.dispatcher.dispatch(Action::ToggleViewSelect);
        }
    }
}

impl View for ViewSelect {
    fn id(&self) -> ViewID {
        ViewID::ViewSelect
    }
}

impl EventHandler for ViewSelect {
    fn process_event(&mut self, evt: &Event, state: &State) -> bool {
        if !state.render_view_select {
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
                        KeyCode::Esc => {
                            if state.render_view_select {
                                self.dispatcher.dispatch(Action::ToggleViewSelect);
                                handled = true;
                            }
                        }
                        KeyCode::Enter => {
                            self.handle_selected();
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

impl WidgetRef for ViewSelect {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();
        self.table.borrow().render_ref(area, buf, &state);
    }
}
