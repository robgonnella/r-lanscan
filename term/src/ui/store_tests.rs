use nanoid::nanoid;
use std::fs;

use super::*;

fn setup() -> (Store, String) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let conf_manager = Arc::new(Mutex::new(
        ConfigManager::new(user.clone(), identity.clone(), tmp_path.as_str()).unwrap(),
    ));
    let current_config = Config::new(user, identity);
    let store = Store::new(conf_manager, current_config);
    (store, tmp_path)
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_new() {
    let (store, conf_path) = setup();
    assert!(store.state.lock().is_ok());
    tear_down(conf_path);
}
