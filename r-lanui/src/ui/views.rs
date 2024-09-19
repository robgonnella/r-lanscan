use ratatui::{crossterm::event::Event, widgets::WidgetRef};

pub mod config;
pub mod device;
pub mod devices;

pub trait EventHandler {
    fn process_event(&mut self, evt: &Event) -> bool;
}

pub trait View: EventHandler + WidgetRef {}
