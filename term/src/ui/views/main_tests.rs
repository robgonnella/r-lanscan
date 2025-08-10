use insta::assert_snapshot;
use nanoid::nanoid;
use pnet::util::MacAddr;
use r_lanlib::scanners::{DeviceWithPorts, Port};
use ratatui::{backend::TestBackend, Terminal};
use std::{
    collections::HashSet,
    fs,
    sync::{mpsc, Mutex},
};

use crate::config::{Config, ConfigManager};

use super::*;

fn setup() -> (MainView, Arc<Store>, String) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));
    let store = Arc::new(Store::new(conf_manager));
    let config = Config {
        id: "default".to_string(),
        cidr: "192.168.1.1/24".to_string(),
        default_ssh_identity: "id_rsa".to_string(),
        default_ssh_port: "22".to_string(),
        default_ssh_user: "user".to_string(),
        device_configs: HashMap::new(),
        ports: vec![],
        theme: "Blue".to_string(),
    };
    store.dispatch(Action::CreateAndSetConfig(config));

    let mut open_ports: HashSet<Port> = HashSet::new();
    open_ports.insert(Port {
        id: 80,
        service: "http".to_string(),
    });

    let device_1 = DeviceWithPorts {
        hostname: "hostname".to_string(),
        ip: "10.10.10.1".to_string(),
        is_current_host: false,
        mac: MacAddr::default().to_string(),
        open_ports: open_ports.clone(),
        vendor: "mac".to_string(),
    };

    let device_2 = DeviceWithPorts {
        hostname: "dev2_hostname".to_string(),
        ip: "10.10.10.2".to_string(),
        is_current_host: true,
        mac: "ff:ff:ff:ff:ff:ff".to_string(),
        open_ports,
        vendor: "linux".to_string(),
    };

    store.dispatch(Action::AddDevice(device_1.clone()));
    store.dispatch(Action::AddDevice(device_2.clone()));
    let (tx, _rx) = mpsc::channel();
    (MainView::new(Arc::clone(&store), tx), store, tmp_path)
}

fn tear_down(conf_path: String) {
    fs::remove_file(conf_path).unwrap();
}

#[test]
fn test_main_view() {
    let (main_view, store, conf_path) = setup();
    let mut terminal = Terminal::new(TestBackend::new(80, 15)).unwrap();
    let state = store.get_state();
    let channel = std::sync::mpsc::channel();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state,
                app_area: frame.area(),
                events: channel.0,
            };

            main_view.render_ref(frame.area(), frame.buffer_mut(), &ctx);
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
    tear_down(conf_path);
}
