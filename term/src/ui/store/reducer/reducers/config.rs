use std::sync::{Arc, Mutex};

use crate::{
    config::{Config, ConfigManager, DeviceConfig},
    ui::{
        colors::{Colors, Theme},
        store::state::State,
    },
};

pub fn update_config(
    prev_state: State,
    config: Config,
    config_manager: &Arc<Mutex<ConfigManager>>,
) -> State {
    let mut state = prev_state.clone();
    let mut manager = config_manager.lock().unwrap();
    manager.update_config(config.clone());
    state.config = config;
    state
}

pub fn set_config(
    prev_state: State,
    config_id: String,
    config_manager: &Arc<Mutex<ConfigManager>>,
) -> State {
    let mut state = prev_state.clone();
    if let Some(conf) = config_manager.lock().unwrap().get_by_id(config_id.as_str()) {
        let theme = Theme::from_string(&conf.theme);
        state.config = conf;
        state.colors = Colors::new(
            theme.to_palette(state.true_color_enabled),
            state.true_color_enabled,
        );
    }
    state
}

pub fn create_and_set_config(
    prev_state: State,
    config: Config,
    config_manager: &Arc<Mutex<ConfigManager>>,
) -> State {
    let mut state = prev_state.clone();
    let mut manager = config_manager.lock().unwrap();
    manager.create(&config);
    let theme = Theme::from_string(&config.theme);
    state.config = config.clone();
    state.colors = Colors::new(
        theme.to_palette(state.true_color_enabled),
        state.true_color_enabled,
    );
    state
}

pub fn update_device_config(
    prev_state: State,
    device_config: DeviceConfig,
    config_manager: &Arc<Mutex<ConfigManager>>,
) -> State {
    let mut state = prev_state.clone();
    let mut config = state.config.clone();
    config
        .device_configs
        .insert(device_config.id.clone(), device_config);
    let mut manager = config_manager.lock().unwrap();
    manager.update_config(config.clone());
    state.config = config;
    state
}
