//! OUI lookup for resolving MAC address prefixes to organization names.
use std::{sync::Arc, time::Duration};

use directories::ProjectDirs;

use crate::{
    error::{RLanLibError, Result},
    oui::{db::OuiDb, offline_db::OfflineOuiDb, traits::Oui},
};

/// OUI (Organizationally Unique Identifier) lookup for MAC addresses.
///
/// Downloads and caches IEEE OUI CSV data locally, then resolves a MAC
/// address prefix to the registered organization name.
pub mod db;
/// Offline database used when unable to download IEEE data files from site
pub mod offline_db;
/// [`Oui`] trait definition and test mocks.
pub mod traits;
/// Data types used by the OUI database.
pub mod types;

/// Initializes a default Oui DB using provided project name and max age
pub fn default(project_name: &str, max_age: Duration) -> Result<Arc<dyn Oui>> {
    log::info!("initializing oui data dir");

    let project_dirs =
        ProjectDirs::from("", "", project_name).ok_or(RLanLibError::Oui(
            format!("failed to find \"{project_name}\" oui data directory"),
        ))?;

    let data_dir = project_dirs.data_dir();

    std::fs::create_dir_all(data_dir).map_err(|e| {
        RLanLibError::Oui(format!(
            "failed to initialize oui data directory: {} : {}",
            data_dir.display(),
            e
        ))
    })?;

    let mut oui = OuiDb::new(data_dir);
    let oui_age = oui.age();

    if let Some(age) = oui_age
        && let Ok(elapsed) = age.elapsed()
        && elapsed > max_age
    {
        log::info!("oui data files are out of date: updating...");
        match oui.update() {
            Ok(_) => log::info!("successfully downloaded vendor data"),
            Err(err) => {
                log::warn!("failed to download vendor data: {}", err);
                log::warn!(
                    "continuing with use of previously downloaded data: vendor data may be slightly out-of-date"
                );
            }
        }
    } else if oui_age.is_none() {
        log::info!("downloading oui data files to {}", data_dir.display());
        match oui.update() {
            Ok(_) => log::info!("successfully downloaded vendor data"),
            Err(err) => {
                log::warn!("failed to download vendor data: {}", err);
                log::warn!(
                    "loading offline data: vendor data may be out-of-date"
                );
                return Ok(Arc::new(OfflineOuiDb));
            }
        }
    }

    log::info!("loading oui data...");
    oui.load_data()?;

    Ok(Arc::new(oui))
}
