use nanoid::nanoid;
use ratatui::backend::TestBackend;
use std::{
    fs,
    sync::{Mutex, mpsc},
};

use crate::{
    config::{Config, ConfigManager},
    ui::store::Store,
};

use super::*;

fn setup() -> (String, Arc<Store>, CrossTermRenderer) {
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
    let config = Config::new(user, identity, cidr);
    let store = Arc::new(Store::new(conf_manager, config));
    let (tx, rx) = mpsc::channel();
    let stdout = io::stdout();
    let real_terminal = Terminal::new(CrosstermBackend::new(stdout)).unwrap();
    let test_terminal = Terminal::new(TestBackend::new(80, 40)).unwrap();
    let renderer = CrossTermRenderer::new_test(
        tx,
        rx,
        real_terminal,
        test_terminal,
        Theme::Blue,
        Arc::clone(&store),
    );
    (tmp_path, store, renderer)
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_app() {
    let (conf_path, _store, app) = setup();
    let _ = app.launch();
    tear_down(conf_path);
}
