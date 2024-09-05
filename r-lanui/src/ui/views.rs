use ratatui::{crossterm::event::Event, Frame};

pub mod config;
pub mod device;
pub mod devices;

pub trait View {
    fn render(&mut self, f: &mut Frame);
    fn process_event(&mut self, evt: &Event) -> bool;
}
