#[cfg(test)]
use mockall::automock;

use color_eyre::eyre::Result;

/// Network scanner trait for easier mocking in tests
#[cfg_attr(test, automock)]
pub trait NetworkMonitor {
    fn monitor(&self) -> Result<()>;
}
