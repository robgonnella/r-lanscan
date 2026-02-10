use insta::assert_snapshot;
use pnet::util::MacAddr;
use r_lanlib::scanners::Port;
use ratatui::{Terminal, backend::TestBackend};
use std::{collections::HashSet, net::Ipv4Addr};

use crate::store::{Dispatcher, StateGetter, Store, reducer::StoreReducer};

use super::*;

fn setup() -> (DeviceView, Store) {
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();

    let store = Store::new(State::default(), StoreReducer::boxed());

    let mut open_ports: HashSet<Port> = HashSet::new();

    open_ports.insert(Port {
        id: 80,
        service: "http".to_string(),
    });

    let device = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: open_ports.into(),
        vendor: "mac".to_string(),
    };

    let device_config = DeviceConfig {
        id: device.mac.to_string(),
        ssh_identity_file: identity,
        ssh_port: 22,
        ssh_user: user,
    };

    store.dispatch(Action::AddDevice(device.clone()));
    (DeviceView::new(device, device_config), store)
}

#[test]
fn test_device_view() {
    let (dev_view, store) = setup();
    let mut terminal = Terminal::new(TestBackend::new(130, 15)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };

            dev_view
                .render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}
