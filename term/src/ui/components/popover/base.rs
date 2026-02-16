//! Utilities for positioning popover dialogs.

use color_eyre::eyre::Result;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, Padding, Widget},
};

use crate::ui::views::traits::{CustomWidgetContext, CustomWidgetRef};

pub struct Popover<'a> {
    content: &'a dyn CustomWidgetRef,
    border_color: Option<Color>,
    width: Option<u16>,
    height: Option<u16>,
}

impl<'a> Popover<'a> {
    pub fn new(content: &'a dyn CustomWidgetRef) -> Self {
        Self {
            content,
            border_color: None,
            width: None,
            height: None,
        }
    }

    /// Calculates a centered popover area within the given parent area.
    pub fn get_popover_area(
        area: Rect,
        percent_x: u16,
        percent_y: u16,
    ) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
            .flex(Flex::Center);
        let horizontal =
            Layout::horizontal([Constraint::Percentage(percent_x)])
                .flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [mut area] = horizontal.areas(area);
        // subtract a little on y axis to move the popover up a little. This is
        // visually a little more pleasing than when perfectly centered.
        area.y = area.y.saturating_sub(8);
        area
    }

    #[allow(dead_code)]
    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    pub fn height(mut self, h: u16) -> Self {
        self.height = Some(h);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }
}

impl<'a> CustomWidgetRef for Popover<'a> {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let pop_area = Self::get_popover_area(
            area,
            self.width.unwrap_or(50),
            self.height.unwrap_or(50),
        );

        let border_color =
            self.border_color.unwrap_or(ctx.state.colors.border_color);

        let block = Block::bordered()
            .border_type(BorderType::Double)
            .border_style(
                Style::new().fg(border_color).bg(ctx.state.colors.buffer_bg),
            )
            .padding(Padding::uniform(2))
            .style(Style::default().bg(ctx.state.colors.buffer_bg));

        let inner_area = block.inner(pop_area);

        Clear.render(pop_area, buf);
        block.render(pop_area, buf);
        self.content.render_ref(inner_area, buf, ctx)
    }
}

#[cfg(test)]
#[path = "./base_tests.rs"]
mod tests;
