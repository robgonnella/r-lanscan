//! Pure config state reducers.

use crate::{
    config::{Config, DeviceConfig},
    ui::{
        colors::{Colors, Theme},
        store::state::State,
    },
};

/// Updates the current config state.
pub fn update_config(state: &mut State, config: &Config) {
    state.theme = Theme::from_string(&config.theme);
    state.config = config.clone();
}

/// Sets a new config as current and applies its theme.
pub fn create_and_set_config(state: &mut State, config: &Config) {
    let theme = Theme::from_string(&config.theme);
    state.config = config.clone();
    state.theme = theme;
    state.colors = Colors::new(
        theme.to_palette(state.true_color_enabled),
        state.true_color_enabled,
    );
}

/// Updates SSH config for a specific device. Returns the updated config for
/// persistence.
pub fn update_device_config(
    state: &mut State,
    device_config: DeviceConfig,
) -> Config {
    let mut config = state.config.clone();
    config
        .device_configs
        .insert(device_config.id.clone(), device_config);
    state.config = config.clone();
    config
}
