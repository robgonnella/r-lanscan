use insta::assert_snapshot;
use nanoid::nanoid;
use ratatui::{Terminal, backend::TestBackend};
use std::{collections::HashMap, fs, sync::Mutex};

use crate::{
    config::{Config, ConfigManager},
    ui::store::Store,
};

use super::*;

fn setup() -> (ConfigView, Arc<Store>, String) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let conf_manager = Arc::new(Mutex::new(
        ConfigManager::new(user, identity, tmp_path.as_str()).unwrap(),
    ));
    let config = Config {
        id: "default".to_string(),
        cidr: "192.168.1.1/24".to_string(),
        default_ssh_identity: "id_rsa".to_string(),
        default_ssh_port: 22,
        default_ssh_user: "user".to_string(),
        device_configs: HashMap::new(),
        ports: vec![],
        theme: "Blue".to_string(),
    };
    let theme = Theme::from_string(&config.theme);
    let store = Arc::new(Store::new(conf_manager, config.clone()));
    store.dispatch(Action::CreateAndSetConfig(config));
    (
        ConfigView::new(Arc::clone(&store) as Arc<dyn Dispatcher>, theme),
        store,
        tmp_path,
    )
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_config_view() {
    let (conf_view, store, conf_path) = setup();
    let mut terminal = Terminal::new(TestBackend::new(80, 15)).unwrap();
    let state = store.get_state().unwrap();
    let channel = std::sync::mpsc::channel();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
                events: channel.0,
            };

            conf_view.render_ref(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
    tear_down(conf_path);
}
