use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table as RatatuiTable, TableState},
};
use unicode_width::UnicodeWidthStr;

use crate::ui::store::store::Colors;

pub const ITEM_HEIGHT: usize = 4;
pub const COLUMN_MAX_WIDTH: u16 = 50;
const ELLIPSIS: &str = "…";

pub struct Table<'c> {
    headers: Vec<String>,
    items: Vec<Vec<String>>,
    colors: &'c Colors,
}

impl<'c> Table<'c> {
    pub fn new(items: Vec<Vec<String>>, headers: Vec<String>, colors: &'c Colors) -> Self {
        Self {
            headers,
            items,
            colors,
        }
    }
}

impl<'c> StatefulWidget for Table<'c> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg)
            .add_modifier(Modifier::BOLD);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_style_fg);

        let header: Row<'_> = self
            .headers
            .iter()
            .map(|h| Cell::from(h.clone()))
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let col_width = area.width / self.headers.len() as u16;
            let item = fit_to_width(data, col_width as usize);
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(ITEM_HEIGHT as u16)
        });

        let spacer = "  ".to_string();
        let spacers = self
            .headers
            .iter()
            .map(|_| Line::from(spacer.clone()))
            .collect::<Vec<Line>>();

        let widths = self
            .headers
            .iter()
            .map(|_| Constraint::Max(COLUMN_MAX_WIDTH))
            .collect::<Vec<Constraint>>();

        let t = RatatuiTable::new(rows, widths)
            .header(header)
            .highlight_style(selected_style)
            .highlight_symbol(Text::from(spacers))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        t.render(area, buf, state)
    }
}

fn fit_to_width(item: &Vec<String>, col_width: usize) -> Vec<String> {
    item.iter()
        .map(|i| {
            let width = i.width();
            let mut value = i.clone();
            if width >= col_width {
                value.truncate(col_width - 10);
                value.push_str(ELLIPSIS);
            }
            value
        })
        .collect::<Vec<String>>()
}
