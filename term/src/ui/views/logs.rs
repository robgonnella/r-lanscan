//! Scrollable view for displaying application logs.
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::{ScrollDirection, ScrollbarState},
};
use std::cell::RefCell;

use crate::ui::{
    components::{
        header::Header, scrollview::ScrollView, table::DEFAULT_ITEM_HEIGHT,
    },
    store::state::{State, ViewID},
    views::traits::CustomStatefulWidget,
};

use super::traits::{
    CustomWidget, CustomWidgetContext, CustomWidgetRef, EventHandler, View,
};

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

    fn render_label(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let header = Header::new("Logs".to_string());
        header.render(area, buf, ctx);
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

        let view = ScrollView::new(&text);

        view.render(area, buf, &mut scroll_state, ctx);
    }
}

impl View for LogsView {
    fn id(&self) -> ViewID {
        ViewID::Logs
    }

    fn legend(&self, _state: &State) -> &str {
        ""
    }

    fn override_main_legend(&self, _state: &State) -> bool {
        false
    }
}

impl CustomWidgetRef for LogsView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let [label_area, _, logs_area] = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // logs
        ])
        .areas(area);

        let label_rects =
            Layout::horizontal([Constraint::Length(20)]).split(label_area);

        self.render_label(label_rects[0], buf, ctx);
        self.render_logs(logs_area, buf, ctx);
    }
}

impl EventHandler for LogsView {
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if ctx.state.render_view_select {
            return false;
        }

        let mut handled = false;

        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(m) => {
                if m.kind == MouseEventKind::ScrollDown {
                    if !ctx.state.logs.is_empty() {
                        self.scroll_state
                            .borrow_mut()
                            .scroll(ScrollDirection::Forward);
                    }
                    handled = true;
                }

                if m.kind == MouseEventKind::ScrollUp {
                    if !ctx.state.logs.is_empty() {
                        self.scroll_state
                            .borrow_mut()
                            .scroll(ScrollDirection::Backward);
                    }
                    handled = true;
                }
            }
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

        handled
    }
}

#[cfg(test)]
#[path = "./logs_tests.rs"]
mod tests;
