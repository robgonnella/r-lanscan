use insta::assert_snapshot;
use r_lanlib::scanners::Device;
use ratatui::{Terminal, backend::TestBackend};
use std::net::Ipv4Addr;

use crate::store::{
    Dispatcher, StateGetter, Store, action::Action, reducer::StoreReducer,
    state::State,
};

use super::*;

fn setup_with_gateway() -> (TopologyView, Store) {
    let store = Store::new(State::default(), StoreReducer::boxed());

    // Gateway at 4ms (simulates WiFi baseline)
    store.dispatch(Action::AddDevice(Device {
        ip: Ipv4Addr::new(192, 168, 1, 1),
        vendor: "Netgear, Inc.".into(),
        latency_ms: Some(4),
        is_gateway: true,
        ..Device::default()
    }));

    // Direct device (4ms raw → 0ms normalized); Linux/macOS TTL
    store.dispatch(Action::AddDevice(Device {
        ip: Ipv4Addr::new(192, 168, 1, 10),
        hostname: "laptop.local".into(),
        vendor: "Apple, Inc.".into(),
        latency_ms: Some(4),
        is_current_host: true,
        response_ttl: Some(63),
        ..Device::default()
    }));

    // Near device (8ms raw → 4ms normalized); Windows TTL
    store.dispatch(Action::AddDevice(Device {
        ip: Ipv4Addr::new(192, 168, 1, 50),
        hostname: "pi-hole".into(),
        vendor: "Raspberry Pi Foundation".into(),
        latency_ms: Some(8),
        response_ttl: Some(127),
        ..Device::default()
    }));

    // Far device (20ms raw → 16ms normalized); no SYN scan yet
    store.dispatch(Action::AddDevice(Device {
        ip: Ipv4Addr::new(192, 168, 1, 100),
        vendor: "Espressif Inc.".into(),
        latency_ms: Some(20),
        ..Device::default()
    }));

    (TopologyView::new(), store)
}

fn setup_no_gateway() -> (TopologyView, Store) {
    let store = Store::new(State::default(), StoreReducer::boxed());

    store.dispatch(Action::AddDevice(Device {
        ip: Ipv4Addr::new(10, 0, 0, 1),
        hostname: "server".into(),
        vendor: "Dell Inc.".into(),
        latency_ms: Some(1),
        ..Device::default()
    }));

    store.dispatch(Action::AddDevice(Device {
        ip: Ipv4Addr::new(10, 0, 0, 2),
        hostname: "workstation".into(),
        vendor: "HP Inc.".into(),
        latency_ms: Some(15),
        ..Device::default()
    }));

    (TopologyView::new(), store)
}

fn setup_empty() -> (TopologyView, Store) {
    let store = Store::new(State::default(), StoreReducer::boxed());
    (TopologyView::new(), store)
}

#[test]
fn test_topology_view_with_gateway() {
    let (view, store) = setup_with_gateway();
    let mut terminal = Terminal::new(TestBackend::new(130, 20)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };
            view.render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_topology_view_no_gateway() {
    let (view, store) = setup_no_gateway();
    let mut terminal = Terminal::new(TestBackend::new(130, 15)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };
            view.render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn test_topology_view_empty() {
    let (view, store) = setup_empty();
    let mut terminal = Terminal::new(TestBackend::new(130, 10)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };
            view.render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap();
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}
