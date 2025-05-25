use ratatui::layout::{Constraint, Flex, Layout, Rect};

pub fn get_popover_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::get_popover_area;

    #[test]
    fn test_get_popover_area() {
        let area = Rect::new(0, 0, 10, 10);
        let expected_area = Rect::new(3, 3, 5, 5);
        let result = get_popover_area(area, 50, 50);
        assert_eq!(result, expected_area);
    }
}
