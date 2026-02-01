//! Side effects returned by the reducer for execution by the store.

use crate::config::Config;

/// Side effects that the reducer requests to be performed after state updates.
///
/// This keeps the reducer pure by separating state computation from I/O
/// operations like file writes.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// No side effect needed.
    None,
    /// Persist a new config to disk.
    CreateConfig(Config),
    /// Persist config updates to disk.
    SaveConfig(Config),
}
