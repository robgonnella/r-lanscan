use std::{cell::RefCell, sync::Arc};

use itertools::Itertools;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::ui::{
    components::table::{self, Table},
    store::{action::Action, state::ViewID, Store},
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

pub struct ViewSelect {
    store: Arc<Store>,
    view_ids: Vec<ViewID>,
    table: RefCell<Table>,
}

impl ViewSelect {
    pub fn new(view_ids: Vec<ViewID>, padding: usize, store: Arc<Store>) -> Self {
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
            store,
            view_ids,
            table: RefCell::new(table_select),
        }
    }

    fn next(&self) {
        self.table.borrow_mut().next();
    }

    fn previous(&self) {
        self.table.borrow_mut().previous();
    }

    fn handle_selected(&self) {
        let i = self.table.borrow().selected();
        if let Some(selected) = i {
            let id = self.view_ids[selected].clone();
            self.store.dispatch(Action::UpdateView(id));
            self.store.dispatch(Action::ToggleViewSelect);
        }
    }
}

impl View for ViewSelect {
    fn id(&self) -> ViewID {
        ViewID::ViewSelect
    }
}

impl EventHandler for ViewSelect {
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if !ctx.state.render_view_select {
            return false;
        }

        let mut handled = false;

        if let Event::Key(key) = evt
            && key.kind == KeyEventKind::Press {
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
                        if ctx.state.render_view_select {
                            self.store.dispatch(Action::ToggleViewSelect);
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

        handled
    }
}

impl CustomWidgetRef for ViewSelect {
    fn render_ref(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        self.table.borrow().render_ref(area, buf, ctx);
    }
}

#[cfg(test)]
#[path = "./view_select_tests.rs"]
mod tests;
