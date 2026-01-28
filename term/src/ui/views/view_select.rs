//! View selection menu for switching between different views.

use std::{cell::RefCell, sync::Arc};

use itertools::Itertools;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::ui::{
    components::table::{self, Table},
    store::{Dispatcher, action::Action, state::ViewID},
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

/// Popover menu for selecting which view to display.
pub struct ViewSelect {
    dispatcher: Arc<dyn Dispatcher>,
    view_ids: Vec<ViewID>,
    table: RefCell<Table>,
}

impl ViewSelect {
    /// Creates a new view selector with the given view options.
    pub fn new(view_ids: Vec<ViewID>, dispatcher: Arc<dyn Dispatcher>) -> Self {
        let spacer = String::from("  ");

        let table_items = view_ids
            .clone()
            .iter()
            .map(|v| vec![format!("{}{}", spacer, v.to_string())])
            .collect_vec();

        let mut table_select =
            Table::new(table_items, None, vec![25], table::DEFAULT_ITEM_HEIGHT);

        table_select.next();

        Self {
            dispatcher,
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
            let id = self.view_ids[selected];
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
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if !ctx.state.render_view_select {
            return false;
        }

        let mut handled = false;

        if let Event::Key(key) = evt
            && key.kind == KeyEventKind::Press
        {
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
