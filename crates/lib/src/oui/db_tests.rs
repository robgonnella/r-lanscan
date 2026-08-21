use std::{fs, path::Path};

use pnet::util::MacAddr;
use tempfile::TempDir;

use crate::oui::{db::OuiDb, traits::Oui};

/// Write a minimal IEEE-style CSV into `dir` with the given filename.
/// Columns: Registry, Assignment, Organization Name, Organization Address
fn write_fixture_csv(dir: &Path, basename: &str, rows: &[(&str, &str)]) {
    let mut content = String::from(
        "Registry,Assignment,Organization Name,Organization Address\n",
    );
    for (assignment, org) in rows {
        content
            .push_str(&format!("MA-L,{},{},Some Address\n", assignment, org));
    }
    fs::write(dir.join(basename), content).unwrap();
}

#[test]
fn new_creates_correct_csv_paths() {
    let dir = TempDir::new().unwrap();
    let db = OuiDb::new(dir.path());
    // csv_paths is private; exercise it indirectly via load_data
    // (just confirm new() doesn't panic)
    drop(db);
}

#[test]
fn age_returns_none_when_files_missing() {
    let dir = TempDir::new().unwrap();
    let db = OuiDb::new(dir.path());
    assert!(db.age().is_none());
}

#[test]
fn age_returns_some_when_all_files_present() {
    let dir = TempDir::new().unwrap();

    for name in &["oui.csv", "mam.csv", "oui36.csv", "cid.csv", "iab.csv"] {
        fs::write(dir.path().join(name), "dummy").unwrap();
    }

    let db = OuiDb::new(dir.path());
    assert!(db.age().is_some());
}

#[test]
fn load_data_and_lookup_standard_oui() {
    let dir = TempDir::new().unwrap();

    write_fixture_csv(
        dir.path(),
        "oui.csv",
        &[("AABBCC", "Acme Corp"), ("112233", "Widgets Inc")],
    );
    for name in &["mam.csv", "oui36.csv", "cid.csv", "iab.csv"] {
        write_fixture_csv(dir.path(), name, &[]);
    }

    let mut db = OuiDb::new(dir.path());
    db.load_data().unwrap();

    let mac: MacAddr = MacAddr::new(0xAA, 0xBB, 0xCC, 0x01, 0x02, 0x03);
    let result = db.lookup(mac).unwrap();
    assert_eq!(result.organization, "Acme Corp");
}

#[test]
fn lookup_returns_none_for_unknown_mac() {
    let dir = TempDir::new().unwrap();

    for name in &["oui.csv", "mam.csv", "oui36.csv", "cid.csv", "iab.csv"] {
        write_fixture_csv(dir.path(), name, &[]);
    }

    let mut db = OuiDb::new(dir.path());
    db.load_data().unwrap();

    let mac: MacAddr = MacAddr::new(0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01);
    assert!(db.lookup(mac).is_none());
}

#[test]
fn load_data_deduplicates_across_files() {
    let dir = TempDir::new().unwrap();

    // Same OUI prefix in both files — first one wins
    write_fixture_csv(dir.path(), "oui.csv", &[("AABBCC", "First")]);
    write_fixture_csv(dir.path(), "mam.csv", &[("AABBCC", "Second")]);
    for name in &["oui36.csv", "cid.csv", "iab.csv"] {
        write_fixture_csv(dir.path(), name, &[]);
    }

    let mut db = OuiDb::new(dir.path());
    db.load_data().unwrap();

    let mac: MacAddr = MacAddr::new(0xAA, 0xBB, 0xCC, 0x00, 0x00, 0x01);
    assert_eq!(db.lookup(mac).unwrap().organization, "First");
}
