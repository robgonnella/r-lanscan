use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use color_eyre::eyre::{Result, eyre};

use crate::{
    config::{Config, ConfigManager},
    ui::{colors::Theme, store::action::Action},
};

pub mod action;
pub mod derived;
pub mod reducer;
pub mod state;

/**
 * Manages the state of our application
 */
pub struct Store {
    state: Mutex<state::State>,
    reducer: reducer::Reducer,
}

impl Store {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>, current_config: Config) -> Self {
        let true_color_enabled = match supports_color::on(supports_color::Stream::Stdout) {
            Some(support) => support.has_16m,
            _ => false,
        };

        let theme = Theme::from_string(&current_config.theme);

        let colors = crate::ui::colors::Colors::new(
            theme.to_palette(true_color_enabled),
            true_color_enabled,
        );

        Self {
            reducer: reducer::Reducer::new(config_manager),
            state: Mutex::new(state::State {
                true_color_enabled,
                ui_paused: false,
                error: None,
                render_view_select: false,
                view_id: state::ViewID::Devices,
                config: current_config,
                arp_history: HashMap::new(),
                devices: Vec::new(),
                device_map: HashMap::new(),
                selected_device: None,
                selected_device_config: None,
                colors,
                message: None,
                cmd_in_progress: None,
                cmd_output: None,
            }),
        }
    }

    pub fn get_state(&self) -> Result<state::State> {
        let state = self
            .state
            .lock()
            .map_err(|e| eyre!("failed to get lock on state: {}", e))?;
        Ok(state.clone())
    }
}

pub trait Dispatcher {
    fn dispatch(&self, action: Action);
}

impl Dispatcher for Store {
    fn dispatch(&self, action: action::Action) {
        if let Ok(mut state) = self.state.lock() {
            self.reducer.reduce(&mut state, action);
        }
    }
}

#[cfg(test)]
#[path = "./store_tests.rs"]
mod tests;
