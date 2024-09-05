use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Cell, HighlightSpacing, Row, Table as RatatuiTable, TableState},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::ui::store::store::Colors;

use super::Component;

pub const ITEM_HEIGHT: usize = 4;
pub const COLUMN_MAX_WIDTH: u16 = 50;
const ELLIPSIS: &str = "…";

pub struct Table<'t> {
    headers: Vec<String>,
    items: Vec<Vec<String>>,
    table_state: &'t mut TableState,
}

impl<'t> Table<'t> {
    pub fn new(
        items: Vec<Vec<String>>,
        headers: Vec<String>,
        table_state: &'t mut TableState,
    ) -> Self {
        Self {
            headers,
            items,
            table_state,
        }
    }
}

impl<'t> Component for Table<'t> {
    fn render(&mut self, f: &mut Frame, area: Rect, colors: &Colors) {
        let header_style = Style::default()
            .fg(colors.header_fg)
            .bg(colors.header_bg)
            .add_modifier(Modifier::BOLD);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(colors.selected_style_fg);

        let header: Row<'_> = self
            .headers
            .iter()
            .map(|h| Cell::from(h.clone()))
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => colors.normal_row_color,
                _ => colors.alt_row_color,
            };
            let col_width = area.width / self.headers.len() as u16;
            let item = fit_to_width(data, col_width as usize);
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(colors.row_fg).bg(color))
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
            .bg(colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        f.render_stateful_widget(t, area, &mut self.table_state);
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
