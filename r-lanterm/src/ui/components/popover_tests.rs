use ratatui::layout::Rect;

use super::get_popover_area;

#[test]
fn test_get_popover_area() {
    let area = Rect::new(0, 0, 10, 10);
    let expected_area = Rect::new(3, 3, 5, 5);
    let result = get_popover_area(area, 50, 50);
    assert_eq!(result, expected_area);
}
