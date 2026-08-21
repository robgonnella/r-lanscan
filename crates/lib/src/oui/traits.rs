use crate::{MacAddr, oui::types::OuiData};

/// Resolves a MAC address to its registered OUI organization.
pub trait Oui: Send + Sync {
    /// Returns the [`OuiData`] for the given MAC address, or `None` if
    /// the prefix is not found in the database.
    fn lookup(&self, mac: MacAddr) -> Option<OuiData>;
}

/// Provides wire mocks for other modules in test
#[cfg(test)]
pub mod mocks {
    use mockall::mock;

    use super::*;

    mock! {
            pub OuiDb {}
            impl Oui for OuiDb {
              fn lookup(&self, mac: MacAddr) -> Option<OuiData>;
            }
    }
}
