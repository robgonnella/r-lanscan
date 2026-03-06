use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::{
    MacAddr,
    error::{RLanLibError, Result},
    oui::{
        traits::Oui,
        types::{OuiData, OuiDataUrl},
    },
};

/// IEEE OUI CSV data sources indexed by assignment type.
const DATA_URLS: [OuiDataUrl; 5] = [
    OuiDataUrl {
        basename: "oui.csv",
        url: "https://standards-oui.ieee.org/oui/oui.csv",
    },
    OuiDataUrl {
        basename: "mam.csv",
        url: "https://standards-oui.ieee.org/oui28/mam.csv",
    },
    OuiDataUrl {
        basename: "oui36.csv",
        url: "https://standards-oui.ieee.org/oui36/oui36.csv",
    },
    OuiDataUrl {
        basename: "cid.csv",
        url: "https://standards-oui.ieee.org/cid/cid.csv",
    },
    OuiDataUrl {
        basename: "iab.csv",
        url: "https://standards-oui.ieee.org/iab/iab.csv",
    },
];

/// Normalises a raw CSV field value by replacing non-breaking spaces
/// and trimming whitespace.
fn clean_string(s: &str) -> String {
    s.replace('\u{00A0}', " ").trim().to_string()
}

/// In-memory OUI database backed by locally cached IEEE CSV files.
pub struct OuiDb {
    data_dir: PathBuf,
    csv_paths: Vec<PathBuf>,
    data: HashMap<String, OuiData>,
}

impl OuiDb {
    /// Creates a new `OuiDb` pointing at `data_dir` for cached CSV files.
    pub fn new(data_dir: &Path) -> Self {
        let mut csv_paths = vec![];

        for data_url in DATA_URLS {
            csv_paths.push(data_dir.join(data_url.basename))
        }

        Self {
            data_dir: data_dir.into(),
            csv_paths,
            data: HashMap::new(),
        }
    }

    /// Returns the modification time of the oldest cached CSV file,
    /// or `None` if any file is missing or its mtime is unavailable.
    pub fn age(&self) -> Option<SystemTime> {
        let mut time = None;

        for data_url in DATA_URLS {
            let file = self.data_dir.join(data_url.basename);

            match fs::metadata(file) {
                Ok(f) => {
                    if let Ok(t) = f.modified() {
                        if time.is_none() {
                            time = Some(t);
                            continue;
                        }

                        if let Some(current_t) = time.as_ref()
                            && t < *current_t
                        {
                            time = Some(t)
                        }
                    } else {
                        return None;
                    }
                }
                Err(_e) => return None,
            }
        }

        time
    }

    /// Loads all cached CSV files into the in-memory lookup table.
    pub fn load_data(&mut self) -> Result<()> {
        let mut used_ouis = HashSet::new();

        for path in &self.csv_paths {
            Self::load_csv(&mut self.data, path, &mut used_ouis)?;
        }

        Ok(())
    }

    /// Downloads fresh OUI data from all IEEE sources and writes them to
    /// `data_dir`.
    pub fn update(&self) -> Result<()> {
        for data_url in DATA_URLS {
            let file_path = self.data_dir.join(data_url.basename);
            let data = Self::request_oui_data(data_url.url)?;

            std::fs::write(&file_path, data).map_err(|e| {
                RLanLibError::Oui(format!(
                    "failed to write oui data: {}: {}",
                    file_path.display(),
                    e,
                ))
            })?;
        }

        Ok(())
    }

    /// Parses a single OUI CSV file and inserts records into `data`,
    /// skipping any duplicate OUI prefixes already present in `used_ouis`.
    fn load_csv(
        data: &mut HashMap<String, OuiData>,
        path: &Path,
        used_ouis: &mut HashSet<String>,
    ) -> Result<()> {
        let mut rdr = csv::Reader::from_path(path).map_err(|e| {
            RLanLibError::Oui(format!(
                "failed to load csv data: {} : {}",
                path.display(),
                e
            ))
        })?;

        for result in rdr.records() {
            let record = result.map_err(|e| {
                RLanLibError::Oui(format!(
                    "failed to get csv record: {} : {}",
                    path.display(),
                    e
                ))
            })?;

            let oui = clean_string(record.get(1).ok_or_else(|| {
                RLanLibError::Oui(format!(
                    "missing OUI field in record: {}",
                    path.display()
                ))
            })?)
            .to_ascii_uppercase();
            let organization =
                clean_string(record.get(2).ok_or_else(|| {
                    RLanLibError::Oui(format!(
                        "missing organization field in record: {}",
                        path.display()
                    ))
                })?);

            if used_ouis.contains(&oui) {
                log::debug!("Discarding duplicate OUI: {oui}: {organization}");
                continue;
            }

            used_ouis.insert(oui.clone());

            data.insert(oui, OuiData { organization });
        }

        Ok(())
    }

    /// Fetches raw CSV text from the given IEEE URL.
    fn request_oui_data(url: &str) -> Result<String> {
        let data = ureq::get(url)
            .call()
            .map_err(|e| {
                RLanLibError::Oui(format!(
                    "failed to request oui data from {url}: {}",
                    e
                ))
            })?
            .body_mut()
            .read_to_string()
            .map_err(|e| {
                RLanLibError::Oui(format!(
                    "failed to read oui response body from {url}: {}",
                    e
                ))
            })?;

        Ok(data)
    }
}

impl Oui for OuiDb {
    /// Retrieve the OUI record for a given MAC address.
    ///
    /// IEEE assigns OUI prefixes at three granularities:
    ///
    /// - **MA-L** (MAC Address Large, `oui.csv`): 24-bit prefix → 6 hex chars
    /// - **MA-M** (MAC Address Medium, `mam.csv`): 28-bit prefix → 7 hex chars
    /// - **MA-S** (MAC Address Small, `oui36.csv`/`iab.csv`): 36-bit prefix
    ///   → 9 hex chars
    ///
    /// We try the most-specific prefix first so that a narrower assignment
    /// (e.g. MA-S) takes precedence over a broader one (e.g. MA-L) for the
    /// same MAC address.
    fn lookup(&self, mac: MacAddr) -> Option<&OuiData> {
        let mut result: Option<&OuiData> = None;

        let mac_str =
            mac.to_string().to_ascii_uppercase().replace([':', '-'], "");

        // MA-S: 36-bit / 9 hex chars
        if mac_str.len() >= 9 {
            result = self.data.get(&mac_str[..9]);
        }

        // MA-M: 28-bit / 7 hex chars
        if mac_str.len() >= 7 {
            result = result.or_else(|| self.data.get(&mac_str[..7]));
        }

        // MA-L: 24-bit / 6 hex chars
        if mac_str.len() >= 6 {
            result = result.or_else(|| self.data.get(&mac_str[..6]));
        }

        result
    }
}

#[cfg(test)]
#[path = "./db_tests.rs"]
mod tests;
