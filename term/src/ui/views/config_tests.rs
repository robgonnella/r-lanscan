use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::HashMap;

use crate::{
    config::Config,
    store::{StateGetter, Store, reducer::StoreReducer},
};

use super::*;

fn setup() -> (ConfigView, Store) {
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

    let store = Store::new(State::default(), StoreReducer::boxed());

    (ConfigView::new(theme), store)
}

#[test]
fn test_config_view() {
    let (conf_view, store) = setup();
    let mut terminal = Terminal::new(TestBackend::new(130, 15)).unwrap();
    let state = store.get_state();

    terminal
        .draw(|frame| {
            let ctx = CustomWidgetContext {
                state: &state,
                app_area: frame.area(),
            };

            conf_view
                .render_ref(frame.area(), frame.buffer_mut(), &ctx)
                .unwrap()
        })
        .unwrap();

    assert_snapshot!(terminal.backend());
}
