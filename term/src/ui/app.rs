//! Main application widget and view router.

use color_eyre::eyre::Result;
use indoc::indoc;
use ratatui::{
    crossterm::event::{
        Event as CrossTermEvent, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Clear as ClearWidget, Padding, Paragraph, Tabs,
        Widget,
    },
};
use std::{cell::RefCell, rc::Rc};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use crate::{
    store::{action::Action, state::State},
    ui::{
        colors::Theme,
        components::{footer::InfoFooter, popover::get_popover_area},
        views::{
            config::ConfigView,
            devices::DevicesView,
            logs::LogsView,
            traits::{
                CustomEventContext, CustomWidget, CustomWidgetContext,
                CustomWidgetRef, EventHandler, View,
            },
        },
    },
};

const LOGO: &str = indoc! {"
▖     ▄▖
▌ ▀▌▛▌▚ ▛▘▀▌▛▌
▙▖█▌▌▌▄▌▙▖█▌▌▌
"};

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "Devices")]
    Devices,
    #[strum(to_string = "Config")]
    Config,
    #[strum(to_string = "Logs")]
    Logs,
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }
}

const DEFAULT_PADDING: Padding = Padding::horizontal(2);

/// Root widget that manages views, handles global events, and renders layout.
pub struct App {
    selected_tab: RefCell<SelectedTab>,
    selected_view: RefCell<Rc<dyn View>>,
    devices_view: Rc<DevicesView>,
    config_view: Rc<ConfigView>,
    logs_view: Rc<LogsView>,
    tabs_area: RefCell<Option<Rect>>,
}

impl App {
    /// Creates a new app with the given theme and dispatcher.
    pub fn new(theme: Theme) -> Self {
        let devices_view = Rc::new(DevicesView::new());
        let config_view = Rc::new(ConfigView::new(theme));
        let logs_view = Rc::new(LogsView::new());
        Self {
            selected_tab: RefCell::new(SelectedTab::Devices),
            selected_view: RefCell::new(devices_view.clone()),
            devices_view,
            config_view,
            logs_view,
            tabs_area: RefCell::new(None),
        }
    }

    fn render_buffer_bg(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
    ) {
        let block = Block::new()
            .style(Style::new().bg(state.colors.buffer_bg))
            .padding(DEFAULT_PADDING);
        block.render(area, buf);
    }

    fn render_top(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let [_padding_top, area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
                .areas(area);

        let [left, middle, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(14),
                Constraint::Min(0),
            ])
            .areas(area);

        let [left_padding, network_and_tabs_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .areas(left);

        let [network_area, tabs_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .areas(network_and_tabs_area);

        let [_, message_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .areas(right);

        Block::new().render(left_padding, buf);

        self.render_network_label(network_area, buf, ctx);

        self.render_tabs(tabs_area, buf, ctx);

        self.render_logo(middle, buf, ctx);

        self.render_message(message_area, buf, ctx);
    }

    fn render_middle_view(
        &self,
        view: &dyn View,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(ctx.state.colors.border_color))
            .border_type(BorderType::Plain)
            .padding(DEFAULT_PADDING);
        let inner_area = block.inner(area);
        block.render(area, buf);
        view.render_ref(inner_area, buf, ctx)
    }

    fn render_tabs(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        // Store the tabs area for click handling
        self.tabs_area.replace(Some(area));

        let titles = SelectedTab::iter().map(|t| {
            Line::from(format!("{:^10}", t)).centered().style(
                Style::new()
                    .fg(ctx.state.colors.text)
                    .bg(ctx.state.colors.gray),
            )
        });
        let selected_tab_index = *self.selected_tab.borrow() as usize;
        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(ctx.state.colors.selected_row_fg);
        Tabs::new(titles)
            .highlight_style(selected_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    fn render_network_label(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let network_str = format!("Network: {}", ctx.state.config.cidr);
        let network_style = Style::default()
            .fg(ctx.state.colors.light_gray)
            .add_modifier(Modifier::BOLD);
        let network =
            Paragraph::new(Line::from(network_str)).style(network_style);

        network.render(area, buf);
    }

    fn render_logo(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        Text::raw(LOGO)
            .fg(ctx.state.colors.selected_row_fg)
            .render(area, buf);
    }

    fn render_message(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if let Some(message) = ctx.state.message.as_ref() {
            let m = Paragraph::new(format!("{}    ", message))
                .alignment(Alignment::Right)
                .fg(ctx.state.colors.light_gray);
            m.render(area, buf);
        }
    }

    fn render_error_popover(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
    ) {
        if let Some(msg) = state.error.as_ref() {
            let block = Block::bordered()
                .border_type(BorderType::Double)
                .border_style(
                    Style::new()
                        .fg(state.colors.error)
                        .bg(state.colors.buffer_bg),
                )
                .padding(Padding::uniform(2))
                .style(Style::default().bg(state.colors.buffer_bg));
            let inner_area = block.inner(area);
            let [msg_area, exit_area] = Layout::vertical([
                Constraint::Percentage(100), // msg
                Constraint::Length(1),       // exit
            ])
            .areas(inner_area);

            let message = Line::from(format!("Error: {}", msg));
            let exit = Paragraph::new("Press enter to clear error").centered();
            ClearWidget.render(area, buf);
            block.render(area, buf);
            message.render(msg_area, buf);
            exit.render(exit_area, buf);
        }
    }

    fn render_info_popover(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
    ) {
        if let Some(msg) = state.popover_message.as_ref() {
            let block = Block::bordered()
                .border_type(BorderType::Double)
                .border_style(
                    Style::new()
                        .fg(state.colors.border_color)
                        .bg(state.colors.buffer_bg),
                )
                .padding(Padding::uniform(2))
                .style(Style::default().bg(state.colors.buffer_bg));
            let inner_area = block.inner(area);

            let message = Paragraph::new(msg.to_string()).centered();
            ClearWidget.render(area, buf);
            block.render(area, buf);
            message.render(inner_area, buf);
        }
    }

    fn render_footer(
        &self,
        legend: &str,
        override_legend: bool,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if override_legend {
            let footer = InfoFooter::new(legend.to_string());
            footer.render(area, buf, ctx);
        } else {
            let mut info =
                String::from("(ctrl-c) quit | (f) next tab | (d) previous tab");

            if !legend.is_empty() {
                info = format!("{info} | {legend}");
            }

            let footer = InfoFooter::new(info);
            footer.render(area, buf, ctx);
        }
    }

    pub fn next_tab(&self) {
        let next = self.selected_tab.borrow().next();
        self.selected_tab.replace(next);
        self.set_next_view();
    }

    pub fn previous_tab(&self) {
        let previous = self.selected_tab.borrow().previous();
        self.selected_tab.replace(previous);
        self.set_next_view();
    }

    pub fn set_next_view(&self) {
        let view: Rc<dyn View> = match *self.selected_tab.borrow() {
            SelectedTab::Devices => self.devices_view.clone(),
            SelectedTab::Config => self.config_view.clone(),
            SelectedTab::Logs => self.logs_view.clone(),
        };

        self.selected_view.replace(view);
    }

    /// Calculates which tab was clicked based on mouse position.
    /// Returns None if the click was outside the tabs area.
    fn calculate_clicked_tab(
        &self,
        click_x: u16,
        tabs_area: Rect,
    ) -> Option<SelectedTab> {
        if click_x < tabs_area.x {
            return None;
        }

        let relative_x = click_x - tabs_area.x;
        // Each tab is 10 chars wide + 1 space divider = 11 chars total
        let tab_width = 11;
        let tab_idx = (relative_x / tab_width) as usize;

        SelectedTab::from_repr(tab_idx)
    }

    /// Selects a specific tab by its enum value.
    fn select_tab(&self, tab: SelectedTab) {
        self.selected_tab.replace(tab);
        self.set_next_view();
    }
}

impl CustomWidgetRef for App {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        // consists of 3 vertical rectangles (top, middle, bottom)
        let page_areas = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

        let view = self.selected_view.borrow();
        let legend = view.legend(ctx.state);
        let override_legend = view.override_main_legend(ctx.state);

        // render background for entire display
        self.render_buffer_bg(area, buf, ctx.state);

        self.render_top(page_areas[0], buf, ctx);

        // view
        self.render_middle_view(view.as_ref(), page_areas[1], buf, ctx)?;

        // legend for current view
        self.render_footer(&legend, override_legend, page_areas[2], buf, ctx);

        // render any popover messages if needed
        self.render_info_popover(
            get_popover_area(area, 50, 15),
            buf,
            ctx.state,
        );

        // popover when there are errors in the store
        // important to render this last so it properly layers on top
        self.render_error_popover(
            get_popover_area(area, 50, 40),
            buf,
            ctx.state,
        );
        // }

        Ok(())
    }
}

impl EventHandler for App {
    fn process_event(
        &self,
        evt: &CrossTermEvent,
        ctx: &CustomEventContext,
    ) -> Result<bool> {
        if ctx.state.error.is_some()
            && let CrossTermEvent::Key(key) = evt
            && key.code == KeyCode::Enter
        {
            ctx.dispatcher.dispatch(Action::SetError(None));
            return Ok(true);
        }

        // scoped so borrow is dropped before any additional event handling
        {
            let view = self.selected_view.borrow();
            let result = view.process_event(evt, ctx);

            if let Err(err) = result {
                ctx.dispatcher
                    .dispatch(Action::SetError(Some(err.to_string())));
                return Ok(true);
            }

            if let Ok(handled) = result
                && handled
            {
                return Ok(handled);
            }
        }

        // Handle mouse clicks on tabs
        if let CrossTermEvent::Mouse(mouse) = evt
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some(tabs_area) = *self.tabs_area.borrow()
            && mouse.row == tabs_area.y
        {
            // Check if click is within tabs row
            let tab_opt = self.calculate_clicked_tab(mouse.column, tabs_area);
            if let Some(tab) = tab_opt {
                self.select_tab(tab);
                return Ok(true);
            }
        }

        if let CrossTermEvent::Key(key) = evt
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('f') | KeyCode::Tab | KeyCode::Right => {
                    self.next_tab();
                    return Ok(true);
                }
                KeyCode::Char('d') | KeyCode::BackTab | KeyCode::Left => {
                    self.previous_tab();
                    return Ok(true);
                }
                _ => {}
            }
        }

        Ok(false)
    }
}

/// Trait combining rendering and event handling for the main application.
pub trait Application: CustomWidgetRef + EventHandler {}

impl Application for App {}

#[cfg(test)]
#[path = "./app_tests.rs"]
mod tests;
