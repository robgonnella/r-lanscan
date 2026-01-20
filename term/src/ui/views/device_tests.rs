use insta::assert_snapshot;
use nanoid::nanoid;
use pnet::util::MacAddr;
use r_lanlib::scanners::Port;
use ratatui::{Terminal, backend::TestBackend};
use std::{
    collections::{HashMap, HashSet},
    fs,
    net::Ipv4Addr,
    sync::Mutex,
};

use crate::{
    config::{Config, ConfigManager},
    ui::store::Store,
};

use super::*;

fn setup() -> (DeviceView, Arc<Store>, String) {
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
    let store = Arc::new(Store::new(conf_manager, config.clone()));

    store.dispatch(Action::CreateAndSetConfig(config));

    let mut open_ports: HashSet<Port> = HashSet::new();
    open_ports.insert(Port {
        id: 80,
        service: "http".to_string(),
    });

    let device = DeviceWithPorts {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports,
        vendor: "mac".to_string(),
    };

    store.dispatch(Action::AddDevice(device.clone()));
    store.dispatch(Action::UpdateSelectedDevice(device.ip));
    (
        DeviceView::new(Arc::clone(&store) as Arc<dyn Dispatcher>),
        store,
        tmp_path,
    )
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_device_view() {
    let (dev_view, store, conf_path) = setup();
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

            dev_view.render_ref(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
    tear_down(conf_path);
}
