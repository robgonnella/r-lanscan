//! Redux-like state container for the terminal UI.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use color_eyre::eyre::{Result, eyre};

use crate::{
    config::{Config, ConfigManager},
    ui::{
        colors::Theme,
        store::{action::Action, state::MAX_LOGS},
    },
};

pub mod action;
pub mod derived;
pub mod reducer;
pub mod state;

/// Centralized state container with thread-safe access.
pub struct Store {
    state: Mutex<state::State>,
    reducer: reducer::Reducer,
}

impl Store {
    /// Creates a new store with the given config manager and initial config.
    pub fn new(
        config_manager: Arc<Mutex<ConfigManager>>,
        current_config: Config,
    ) -> Self {
        let true_color_enabled =
            match supports_color::on(supports_color::Stream::Stdout) {
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
                logs: Vec::with_capacity(MAX_LOGS),
                render_view_select: false,
                view_id: state::ViewID::Devices,
                config: current_config,
                arp_history: HashMap::new(),
                device_map: HashMap::new(),
                sorted_device_list: vec![],
                selected_device: None,
                selected_device_config: None,
                colors,
                message: None,
                cmd_in_progress: None,
                cmd_output: None,
            }),
        }
    }

    /// Returns a clone of the current state.
    pub fn get_state(&self) -> Result<state::State> {
        let state = self
            .state
            .lock()
            .map_err(|e| eyre!("failed to get lock on state: {}", e))?;
        Ok(state.clone())
    }
}

/// Dispatches actions to update application state.
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
