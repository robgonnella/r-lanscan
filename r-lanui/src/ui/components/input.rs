use ratatui::layout::Rect;

use crate::ui::store::store::Colors;

use super::Component;

pub struct Input {}

impl Component for Input {
    fn render(&mut self, _f: &mut ratatui::Frame, _area: Rect, _colors: &Colors) {}
}
