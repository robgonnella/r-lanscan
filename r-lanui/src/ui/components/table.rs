use std::cell::RefCell;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Row, ScrollbarState, StatefulWidget, Table as RatatuiTable,
        TableState,
    },
};
use unicode_width::UnicodeWidthStr;

use crate::ui::{
    store::state::State,
    views::traits::{CustomStatefulWidget, CustomWidgetRef},
};

use super::scrollbar::ScrollBar;

pub const DEFAULT_ITEM_HEIGHT: usize = 3;
pub const COLUMN_MAX_WIDTH: u16 = 50;
const ELLIPSIS: &str = "…";

pub struct Table {
    headers: Option<Vec<String>>,
    items: Vec<Vec<String>>,
    item_height: usize,
    column_sizes: Vec<usize>,
    table_state: RefCell<TableState>,
    scroll_state: RefCell<ScrollbarState>,
}

impl Table {
    pub fn new(
        items: Vec<Vec<String>>,
        headers: Option<Vec<String>>,
        column_sizes: Vec<usize>,
        item_height: usize,
    ) -> Self {
        let mut scroll_height = item_height;

        if items.len() > 0 {
            scroll_height = (items.len() - 1) * item_height;
        }

        Self {
            headers,
            column_sizes,
            items,
            item_height,
            table_state: RefCell::new(TableState::new()),
            scroll_state: RefCell::new(ScrollbarState::new(scroll_height)),
        }
    }

    pub fn update_items(&mut self, items: Vec<Vec<String>>) -> Option<usize> {
        let mut selected: Option<usize> = None;
        let selection_opt = self.table_state.borrow().selected();

        if let Some(current_selected) = selection_opt {
            selected = Some(current_selected);

            if current_selected >= items.len() {
                let new_idx = items.len() - 1;
                selected = Some(new_idx);
                self.table_state.borrow_mut().select(selected);
                let new_scroll_state = self
                    .scroll_state
                    .borrow_mut()
                    .position(new_idx * self.item_height);
                self.scroll_state = RefCell::new(new_scroll_state);
            }
        }

        self.items = items;
        selected
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.borrow().selected()
    }

    pub fn next(&mut self) -> usize {
        let i = match self.table_state.borrow().selected() {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };

        self.table_state.borrow_mut().select(Some(i));

        let new_scroll_state = self
            .scroll_state
            .borrow_mut()
            .position(i * self.item_height);

        self.scroll_state = RefCell::new(new_scroll_state);

        i
    }

    pub fn previous(&mut self) -> usize {
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.table_state.borrow_mut().select(Some(i));

        let new_scroll_state = self.scroll_state.borrow().position(i * self.item_height);

        self.scroll_state = RefCell::new(new_scroll_state);

        i
    }
}

impl CustomWidgetRef for Table {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        // main table view + right aligned scrollbar
        let table_rects =
            Layout::horizontal([Constraint::Min(5), Constraint::Length(3)]).split(area);

        if table_rects[0].width < 1 || table_rects[0].height < 1 {
            return;
        }

        let header = self.headers.as_ref().map(|hs| {
            let header_style = Style::default()
                .fg(state.colors.header_fg)
                .bg(state.colors.header_bg)
                .add_modifier(Modifier::BOLD);

            hs.iter()
                .map(|h| Cell::from(h.clone()))
                .collect::<Row>()
                .style(header_style)
                .height(1)
        });

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(state.colors.selected_row_fg);

        let rows = self.items.iter().enumerate().map(|(_i, data)| {
            let item = fit_to_width(data, self.column_sizes.clone());

            // line break - hacky way of centering the text
            let mut line_break_count = self.item_height / 2;
            let mut line_breaks = String::from("");

            if line_break_count > 1 && line_break_count % 2 == 0 {
                line_break_count -= 1;
            }

            for _ in 0..line_break_count {
                line_breaks += "\n";
            }

            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("{line_breaks}{content}"))))
                .collect::<Row>()
                .style(Style::new().fg(state.colors.row_fg).bg(state.colors.row_bg))
                .height(self.item_height as u16)
        });

        let mut widths: Vec<Constraint> = Vec::new();

        for _ in self.column_sizes.iter() {
            widths.push(Constraint::Max(COLUMN_MAX_WIDTH));
        }

        let mut t = RatatuiTable::new(rows, widths)
            .row_highlight_style(selected_style)
            .bg(state.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        if let Some(h) = header {
            t = t.header(h);
        }

        t.render(table_rects[0], buf, &mut self.table_state.borrow_mut());

        let scrollbar = ScrollBar::new();
        let mut scroll_state = self.scroll_state.borrow_mut();
        scrollbar.render(table_rects[1], buf, &mut scroll_state, state);
    }
}

fn fit_to_width(item: &Vec<String>, col_widths: Vec<usize>) -> Vec<String> {
    item.iter()
        .enumerate()
        .map(|(i, v)| {
            let width = v.width();
            let mut value = v.clone();
            let col_width = col_widths[i];
            if width >= col_width {
                value.truncate(col_width - ELLIPSIS.width());
                value.push_str(ELLIPSIS);
            }
            value
        })
        .collect::<Vec<String>>()
}
