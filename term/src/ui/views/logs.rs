//! Scrollable view for displaying application logs.
use color_eyre::eyre::Result;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{ScrollDirection, ScrollbarState},
};
use std::cell::RefCell;

use crate::ui::{
    components::{scrollview::ScrollView, table::DEFAULT_ITEM_HEIGHT},
    views::traits::{CustomEventContext, CustomStatefulWidget},
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

/// View for editing global application settings.
#[derive(Default)]
pub struct LogsView {
    scroll_state: RefCell<ScrollbarState>,
}

impl LogsView {
    pub fn new() -> Self {
        Self {
            scroll_state: RefCell::new(ScrollbarState::new(
                DEFAULT_ITEM_HEIGHT,
            )),
        }
    }

    pub fn render_logs(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let text = ctx
            .state
            .logs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut scroll_state = self.scroll_state.borrow_mut();

        *scroll_state = scroll_state
            .content_length(ctx.state.logs.len() * DEFAULT_ITEM_HEIGHT)
            .viewport_content_length(area.height.into());

        let view = ScrollView::new(&text).style(
            Style::default()
                .fg(ctx.state.colors.light_gray)
                .add_modifier(Modifier::BOLD),
        );

        view.render(area, buf, &mut scroll_state, ctx);
    }
}

impl View for LogsView {}

impl CustomWidgetRef for LogsView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let [_, logs_area] = Layout::vertical([
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // logs
        ])
        .areas(area);

        self.render_logs(logs_area, buf, ctx);
        Ok(())
    }
}

impl EventHandler for LogsView {
    fn process_event(
        &self,
        evt: &Event,
        ctx: &CustomEventContext,
    ) -> Result<bool> {
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
                            if !ctx.state.logs.is_empty() {
                                self.scroll_state
                                    .borrow_mut()
                                    .scroll(ScrollDirection::Forward);
                            }
                            handled = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if !ctx.state.logs.is_empty() {
                                self.scroll_state
                                    .borrow_mut()
                                    .scroll(ScrollDirection::Backward);
                            }
                            handled = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(handled)
    }
}

#[cfg(test)]
#[path = "./logs_tests.rs"]
mod tests;
