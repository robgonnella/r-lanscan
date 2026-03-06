use std::sync::Arc;

#[cfg(test)]
use mockall::automock;

use color_eyre::eyre::Result;
use r_lanlib::oui::traits::Oui;

/// Network scanner trait for easier mocking in tests
#[cfg_attr(test, automock)]
pub trait NetworkMonitor {
    fn monitor(&self, oui: Arc<dyn Oui>) -> Result<()>;
}
