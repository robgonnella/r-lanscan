//! Pure config state reducers.

use crate::{
    config::{Config, DeviceConfig},
    store::state::State,
    ui::colors::Theme,
};

/// Updates the current config state.
pub fn update_config(state: &mut State, config: &Config) {
    state.theme = Theme::from_string(&config.theme);
    state.config = config.clone();
}

/// Updates SSH config for a specific device. Returns the updated config for
/// persistence.
pub fn update_device_config(state: &mut State, device_config: DeviceConfig) {
    let mut config = state.config.clone();
    config
        .device_configs
        .insert(device_config.id.clone(), device_config);
    state.config = config.clone();
}
