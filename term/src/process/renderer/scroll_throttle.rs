use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};

/// Default duration to throttle scroll events (arrow keys and vi-style j/k).
const DEFAULT_SCROLL_THROTTLE_DURATION: Duration = Duration::from_millis(20);

pub struct ScrollThrottle {
    last_up: RefCell<Instant>,
    last_down: RefCell<Instant>,
    throttle_duration: Duration,
}

impl Default for ScrollThrottle {
    fn default() -> Self {
        Self::new(DEFAULT_SCROLL_THROTTLE_DURATION)
    }
}

impl ScrollThrottle {
    /// Creates a new ScrollThrottle with a custom throttle duration.
    pub fn new(throttle_duration: Duration) -> Self {
        // Initialize timestamps in the past (beyond throttle duration) to
        // ensure first events are never throttled
        let past =
            Instant::now() - throttle_duration - Duration::from_millis(1);
        Self {
            last_up: RefCell::new(past),
            last_down: RefCell::new(past),
            throttle_duration,
        }
    }

    pub fn throttled(&self, evt: &Event) -> bool {
        if self.should_check_up_throttle(evt) {
            let last_up = *self.last_up.borrow();
            if last_up.elapsed() <= self.throttle_duration {
                return true;
            }
            self.last_up.replace(Instant::now());
            return false;
        }

        if self.should_check_down_throttle(evt) {
            let last_down = *self.last_down.borrow();
            if last_down.elapsed() <= self.throttle_duration {
                return true;
            }
            self.last_down.replace(Instant::now());
            return false;
        }

        false
    }

    fn should_check_up_throttle(&self, evt: &Event) -> bool {
        match evt {
            Event::Key(key) => match key.kind {
                KeyEventKind::Press => match key.code {
                    KeyCode::Up => true,
                    KeyCode::Char(c) => c == 'k',
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        }
    }

    fn should_check_down_throttle(&self, evt: &Event) -> bool {
        match evt {
            Event::Key(key) => match key.kind {
                KeyEventKind::Press => match key.code {
                    KeyCode::Down => true,
                    KeyCode::Char(c) => c == 'j',
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "./scroll_throttle_tests.rs"]
mod tests;
