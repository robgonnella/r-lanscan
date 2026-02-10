use insta::assert_snapshot;
use pnet::util::MacAddr;
use r_lanlib::scanners::{Device, Port};
use ratatui::{Terminal, backend::TestBackend};
use std::{collections::HashSet, net::Ipv4Addr};

use crate::store::{
    Dispatcher, StateGetter, Store, action::Action, reducer::StoreReducer,
};

use super::*;

fn setup() -> (DevicesView, Store) {
    let store = Store::new(State::default(), StoreReducer::boxed());

    let mut open_ports: HashSet<Port> = HashSet::new();

    open_ports.insert(Port {
        id: 80,
        service: "http".to_string(),
    });

    let device_1 = Device {
        hostname: "hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 1),
        mac: MacAddr::default(),
        is_current_host: false,
        open_ports: open_ports.clone().into(),
        vendor: "mac".to_string(),
    };

    let device_2 = Device {
        hostname: "dev2_hostname".to_string(),
        ip: Ipv4Addr::new(10, 10, 10, 2),
        mac: MacAddr::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        is_current_host: true,
        open_ports: open_ports.into(),
        vendor: "linux".to_string(),
    };

    store.dispatch(Action::AddDevice(device_1.clone()));
    store.dispatch(Action::AddDevice(device_2.clone()));
    (DevicesView::new(), store)
}

#[test]
fn test_devices_view() {
    let (devs_view, store) = setup();
    let mut terminal = Terminal::new(TestBackend::new(130, 15)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };

            devs_view
                .render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}
