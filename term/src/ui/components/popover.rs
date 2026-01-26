//! Utilities for positioning popover dialogs.

use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// Calculates a centered popover area within the given parent area.
pub fn get_popover_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

#[cfg(test)]
#[path = "./popover_tests.rs"]
mod tests;
