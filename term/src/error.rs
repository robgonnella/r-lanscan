use std::any::Any;

use color_eyre::eyre::{Report, eyre};

pub fn report_from_thread_panic(e: Box<dyn Any + Send>) -> Report {
    if let Some(value) = e.downcast_ref::<&str>() {
        eyre!("thread panicked with {value}")
    } else if let Some(value) = e.downcast_ref::<&String>() {
        eyre!("thread panicked with {value}")
    } else {
        eyre!("thread panicked for unknown reason")
    }
}
