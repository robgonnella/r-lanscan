use crate::oui::{traits::Oui, types::OuiData};

/// Offline Oui database used when unable to download IEEE data files from site
#[derive(Default)]
pub struct OfflineOuiDb;

impl Oui for OfflineOuiDb {
    fn lookup(&self, mac: crate::MacAddr) -> Option<OuiData> {
        oui_data::lookup(&mac.to_string()).map(|d| d.into())
    }
}
