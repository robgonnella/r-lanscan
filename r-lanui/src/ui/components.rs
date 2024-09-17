use ratatui::{layout::Rect, Frame};

use super::store::store::Colors;

pub mod field;
pub mod footer;
pub mod scrollbar;
pub mod table;

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect, colors: &Colors);
}
