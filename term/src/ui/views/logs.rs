//! Scrollable view for displaying application logs.
use color_eyre::eyre::Result;
use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::ScrollbarState,
};
use std::cell::RefCell;

use crate::ui::{
    components::{scrollview::ScrollView, table::DEFAULT_ITEM_HEIGHT},
    views::traits::{CustomEventContext, CustomStatefulWidget},
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

/// View for displaying application logs.
pub struct LogsView {
    scroll_state: RefCell<ScrollbarState>,
    /// Manually-tracked scroll offset (lines from top of content).
    scroll_offset: RefCell<usize>,
    /// When true the view is pinned to the bottom and auto-scrolls as
    /// new logs arrive. Cleared when the user explicitly scrolls up.
    at_bottom: RefCell<bool>,
}

impl LogsView {
    pub fn new() -> Self {
        Self {
            scroll_state: RefCell::new(ScrollbarState::default()),
            scroll_offset: RefCell::new(0),
            // Open pinned to the bottom so the latest log is visible.
            at_bottom: RefCell::new(true),
        }
    }

    fn scroll_down(&self) {
        // at_bottom is updated during the next render pass once we
        // know the real max position.
        self.scroll_offset
            .replace_with(|&mut v| v.saturating_add(1));
    }

    fn scroll_up(&self) {
        // User is explicitly moving away from the bottom.
        self.scroll_offset
            .replace_with(|&mut v| v.saturating_sub(1));
        self.at_bottom.replace(false);
    }

    pub fn render_logs(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let text = ctx.state.logs.iter().map(String::as_str).join("\n\n");

        let content_lines = ctx.state.logs.len() * DEFAULT_ITEM_HEIGHT;
        let viewport_height = area.height as usize;

        // Maximum useful scroll position. Allow DEFAULT_ITEM_HEIGHT
        // extra lines past the natural end so that a last log entry
        // which wraps onto multiple lines remains fully visible.
        let max_pos = if content_lines > viewport_height {
            (content_lines - viewport_height) + DEFAULT_ITEM_HEIGHT
        } else {
            0
        };

        // Auto-scroll: if pinned to the bottom, follow new content.
        if *self.at_bottom.borrow() {
            *self.scroll_offset.borrow_mut() = max_pos;
        }

        // Clamp to the allowed maximum.
        let offset = (*self.scroll_offset.borrow()).min(max_pos);
        *self.scroll_offset.borrow_mut() = offset;

        // Re-evaluate the pin flag so that scrolling back to the
        // bottom re-enables auto-scroll automatically.
        *self.at_bottom.borrow_mut() = offset >= max_pos;

        // Set content_length so that the scrollbar thumb sits at the
        // bottom of the track when position == max_pos. Ratatui uses
        // (content_length - 1) as the maximum position when computing
        // the thumb location, so content_length = max_pos + 1 gives
        // correct bottom-pinned positioning.
        let mut scroll_state = self.scroll_state.borrow_mut();
        *scroll_state = scroll_state
            .content_length(max_pos + 1)
            .viewport_content_length(viewport_height)
            .position(offset);

        let view = ScrollView::new(&text).style(
            Style::default()
                .fg(ctx.state.colors.light_gray)
                .add_modifier(Modifier::BOLD),
        );

        view.render(area, buf, &mut scroll_state, ctx);
    }
}

impl Default for LogsView {
    fn default() -> Self {
        Self::new()
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
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    if !ctx.state.logs.is_empty() {
                        self.scroll_down();
                    }
                    handled = true;
                }
                MouseEventKind::ScrollUp => {
                    if !ctx.state.logs.is_empty() {
                        self.scroll_up();
                    }
                    handled = true;
                }
                _ => {}
            },
            Event::Paste(_s) => {}
            Event::Resize(_x, _y) => {}
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !ctx.state.logs.is_empty() {
                                self.scroll_down();
                            }
                            handled = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if !ctx.state.logs.is_empty() {
                                self.scroll_up();
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
