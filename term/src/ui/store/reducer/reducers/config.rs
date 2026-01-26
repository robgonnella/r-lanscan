//! Config state reducers for persisting app and device settings.

use std::sync::{Arc, Mutex};

use crate::{
    config::{Config, ConfigManager, DeviceConfig},
    ui::{
        colors::{Colors, Theme},
        store::{reducer::reducers::ui::set_error, state::State},
    },
};

/// Updates the current config and persists it to disk.
pub fn update_config(
    state: &mut State,
    config: Config,
    config_manager: &Arc<Mutex<ConfigManager>>,
) {
    match config_manager.lock() {
        Ok(mut manager) => {
            if let Err(err) = manager.update_config(config.clone()) {
                set_error(state, Some(err.to_string()));
            } else {
                state.config = config;
            }
        }
        Err(err) => set_error(state, Some(err.to_string())),
    }
}

/// Loads an existing config by ID and applies its theme.
pub fn set_config(
    state: &mut State,
    config_id: String,
    config_manager: &Arc<Mutex<ConfigManager>>,
) {
    match config_manager.lock() {
        Ok(manager) => {
            if let Some(conf) = manager.get_by_id(&config_id) {
                let theme = Theme::from_string(&conf.theme);
                state.config = conf;
                state.colors = Colors::new(
                    theme.to_palette(state.true_color_enabled),
                    state.true_color_enabled,
                );
            }
        }
        Err(err) => set_error(state, Some(err.to_string())),
    }
}

/// Creates a new config, persists it, and sets it as current.
pub fn create_and_set_config(
    state: &mut State,
    config: Config,
    config_manager: &Arc<Mutex<ConfigManager>>,
) {
    match config_manager.lock() {
        Ok(mut manager) => {
            if let Err(err) = manager.create(&config) {
                set_error(state, Some(err.to_string()))
            } else {
                let theme = Theme::from_string(&config.theme);
                state.config = config.clone();
                state.colors = Colors::new(
                    theme.to_palette(state.true_color_enabled),
                    state.true_color_enabled,
                );
            }
        }
        Err(err) => set_error(state, Some(err.to_string())),
    }
}

/// Updates SSH config for a specific device and persists it.
pub fn update_device_config(
    state: &mut State,
    device_config: DeviceConfig,
    config_manager: &Arc<Mutex<ConfigManager>>,
) {
    let mut config = state.config.clone();
    config
        .device_configs
        .insert(device_config.id.clone(), device_config);
    match config_manager.lock() {
        Ok(mut manager) => {
            if let Err(err) = manager.update_config(config.clone()) {
                set_error(state, Some(err.to_string()));
            } else {
                state.config = config;
            }
        }
        Err(err) => set_error(state, Some(err.to_string())),
    }
}
