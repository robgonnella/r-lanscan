use nanoid::nanoid;
use std::{
    fs,
    sync::{Arc, Mutex},
};

use crate::config::{Config, ConfigManager};

use super::*;

fn setup() -> (Store, String) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();
    let config_manager = ConfigManager::builder()
        .default_user(user.clone())
        .default_identity(identity.clone())
        .default_cidr(cidr.clone())
        .path(tmp_path.clone())
        .build()
        .unwrap();
    let conf_manager = Arc::new(Mutex::new(config_manager));
    let current_config = Config::new(user, identity, cidr);
    let store = Store::new(conf_manager, current_config);
    (store, tmp_path)
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_new() {
    let (store, conf_path) = setup();
    assert!(!store.state.read().unwrap().ui_paused);
    tear_down(conf_path);
}

#[test]
fn test_load_config() {
    let (store, conf_path) = setup();

    // First create a config to load
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let cidr = "192.168.1.1/24".to_string();
    let mut config = Config::new(user, identity, cidr);
    config.id = "test_config".to_string();
    config.theme = "Emerald".to_string();

    store
        .dispatch(action::Action::CreateAndSetConfig(config.clone()))
        .unwrap();

    // Now load it
    store.load_config("test_config").unwrap();

    let state = store.get_state().unwrap();
    assert_eq!(state.config.id, "test_config");
    assert_eq!(state.config.theme, "Emerald");

    tear_down(conf_path);
}
