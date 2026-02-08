//! Scrollable table component with selection support.

use color_eyre::eyre::Result;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Row, ScrollbarState, StatefulWidget,
        Table as RatatuiTable, TableState,
    },
};
use std::cell::RefCell;
use unicode_width::UnicodeWidthStr;

use crate::ui::views::traits::{
    CustomStatefulWidget, CustomWidgetContext, CustomWidgetRef,
};

use super::scrollbar::ScrollBar;

/// Default height for table rows.
pub const DEFAULT_ITEM_HEIGHT: usize = 3;
/// Used for overflow when item exceeds max width
const ELLIPSIS: &str = "…";

/// Scrollable table with optional headers, row selection, and scrollbar.
pub struct Table {
    headers: Option<Vec<String>>,
    items: Vec<Vec<String>>,
    item_height: usize,
    column_sizes: Vec<u16>,
    centering_breaks: String,
    table_state: RefCell<TableState>,
    scroll_state: RefCell<ScrollbarState>,
}

impl Table {
    /// Creates a new table with the given items, optional headers, and column
    /// sizes.
    pub fn new(
        items: Vec<Vec<String>>,
        headers: Option<Vec<String>>,
        column_sizes: Vec<u16>,
        item_height: usize,
    ) -> Self {
        let mut scroll_height = item_height;

        if !items.is_empty() {
            scroll_height = (items.len() - 1) * item_height;
        }

        // line break - hacky way of centering the text in each cell
        let mut line_break_count = item_height / 2;
        let mut line_breaks = String::from("");

        if line_break_count > 1 && line_break_count.is_multiple_of(2) {
            line_break_count -= 1;
        }

        for _ in 0..line_break_count {
            line_breaks += "\n";
        }

        Self {
            headers,
            column_sizes,
            items,
            item_height,
            centering_breaks: line_breaks,
            table_state: RefCell::new(TableState::new()),
            scroll_state: RefCell::new(ScrollbarState::new(scroll_height)),
        }
    }

    /// Updates the table items, adjusting selection if needed. Returns the new
    /// selected index if changed.
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

    /// Returns the currently selected row index, if any.
    pub fn selected(&self) -> Option<usize> {
        self.table_state.borrow().selected()
    }

    /// Moves selection to the next row.
    pub fn next(&mut self) -> usize {
        let i = match self.table_state.borrow().selected() {
            // don't wrap
            Some(i) => {
                if i + 1 > self.items.len() - 1 {
                    self.items.len() - 1
                } else {
                    i + 1
                }
            }
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

    /// Moves selection to the previous row.
    pub fn previous(&mut self) -> usize {
        let i = match self.table_state.borrow().selected() {
            // prevent wrap with saturating_sub
            Some(i) => i.saturating_sub(1),
            None => 0,
        };

        self.table_state.borrow_mut().select(Some(i));

        let new_scroll_state =
            self.scroll_state.borrow().position(i * self.item_height);

        self.scroll_state = RefCell::new(new_scroll_state);

        i
    }
}

impl CustomWidgetRef for Table {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        // main table view + right aligned scrollbar
        let table_rects = Layout::horizontal([
            Constraint::Percentage(100),
            Constraint::Length(3),
        ])
        .split(area);

        let header = self.headers.as_ref().map(|hs| {
            let header_style = Style::default()
                .fg(ctx.state.colors.text)
                .bg(ctx.state.colors.row_header_bg)
                .add_modifier(Modifier::BOLD);

            hs.iter()
                .map(|h| Cell::from(format!(" {h}")))
                .collect::<Row>()
                .style(header_style)
                .height(1)
        });

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(ctx.state.colors.selected_row_fg);

        // uses provided column sizes to calculate the remaining available
        // space for the last column. We use this to allow the last column
        // to fill up all remaining space rather than truncating on the
        // explicit provided final col size
        let mut free_for_last_col = area.width;
        self.column_sizes.iter().enumerate().for_each(|(i, s)| {
            if i != self.column_sizes.len() - 1 {
                free_for_last_col =
                    free_for_last_col.saturating_sub(s.to_owned());
            }
        });

        let rows = self
            .items
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, content)| {
                        // allow the final column to consume the rest of the
                        // available space
                        let max_width = if i == self.column_sizes.len() - 1 {
                            // - app padding - scroll width - extra padding
                            free_for_last_col.saturating_sub(10)
                        } else {
                            self.column_sizes[i]
                        };
                        let formatted_content =
                            fit_to_width(content, max_width);
                        Cell::from(Text::from(format!(
                            "{} {formatted_content}",
                            self.centering_breaks
                        )))
                    })
                    .collect::<Row>()
                    .style(
                        Style::new()
                            .fg(ctx.state.colors.text)
                            .bg(ctx.state.colors.buffer_bg),
                    )
                    .height(self.item_height as u16)
            })
            .collect::<Vec<_>>();

        let constraints = self
            .column_sizes
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if i == self.column_sizes.len() - 1 {
                    Constraint::Min(w.to_owned())
                } else {
                    Constraint::Max(w.to_owned())
                }
            })
            .collect::<Vec<_>>();

        let mut t = RatatuiTable::new(rows, constraints)
            .row_highlight_style(selected_style)
            .bg(ctx.state.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        if let Some(h) = header {
            t = t.header(h);
        }

        t.render(table_rects[0], buf, &mut self.table_state.borrow_mut());

        let scrollbar = ScrollBar::new();
        let mut scroll_state = self.scroll_state.borrow_mut();
        scrollbar.render(table_rects[1], buf, &mut scroll_state, ctx);
        Ok(())
    }
}

fn fit_to_width(content: &str, max_width: u16) -> String {
    let width = content.width() as u16;
    let mut value = content.to_string();
    let ellipsis_width = ELLIPSIS.width() as u16;
    let trim_length = (max_width.saturating_sub(ellipsis_width * 2)) as usize;

    if width >= max_width {
        value.truncate(trim_length);
        value = value.trim().to_string();
        value.push_str(ELLIPSIS);
    }

    value
}

#[cfg(test)]
#[path = "./table_tests.rs"]
mod tests;
