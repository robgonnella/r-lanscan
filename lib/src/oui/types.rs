/// A remote OUI data source with a filename and download URL.
pub(crate) struct OuiDataUrl {
    /// Local filename used when saving the downloaded CSV.
    pub(crate) basename: &'static str,
    /// IEEE URL to download the CSV from.
    pub(crate) url: &'static str,
}

/// OUI record containing the registered organization name.
#[derive(Debug, Clone)]
pub struct OuiData {
    /// Registered organization name for this OUI prefix.
    pub(crate) organization: String,
}

impl OuiData {
    /// Returns the vendor / organization for this OuiData
    pub fn organization(&self) -> &str {
        &self.organization
    }
}

impl From<&oui_data::OuiData> for OuiData {
    fn from(value: &oui_data::OuiData) -> Self {
        Self {
            organization: value.organization().into(),
        }
    }
}
