use ratatui::{crossterm::event::KeyEvent, Frame};

pub mod config;
pub mod device;
pub mod devices;

pub trait View {
    fn render(&mut self, f: &mut Frame);
    fn process_key_event(&mut self, key: KeyEvent) -> bool;
}
