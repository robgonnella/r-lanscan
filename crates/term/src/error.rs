//! Error handling utilities for thread panics.

use std::any::Any;

use color_eyre::eyre::{Report, eyre};

/// Converts a thread panic into a color_eyre Report for consistent error
/// handling.
pub fn report_from_thread_panic(e: Box<dyn Any + Send>) -> Report {
    if let Some(value) = e.downcast_ref::<&str>() {
        eyre!("thread panicked with {value}")
    } else if let Some(value) = e.downcast_ref::<&String>() {
        eyre!("thread panicked with {value}")
    } else {
        eyre!("thread panicked for unknown reason")
    }
}
