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

fn setup() -> (String, Arc<Store>, App) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let conf_manager = Arc::new(Mutex::new(
        ConfigManager::new(user.clone(), identity.clone(), tmp_path.as_str()).unwrap(),
    ));
    let config = Config::new(user, identity);
    let store = Arc::new(Store::new(conf_manager, config));
    let (tx, rx) = mpsc::channel();
    let stdout = io::stdout();
    let real_terminal = Terminal::new(CrosstermBackend::new(stdout)).unwrap();
    let test_terminal = Terminal::new(TestBackend::new(80, 40)).unwrap();
    let app = App::new_test(
        tx,
        rx,
        real_terminal,
        test_terminal,
        Theme::Blue,
        Arc::clone(&store),
    );
    (tmp_path, store, app)
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
