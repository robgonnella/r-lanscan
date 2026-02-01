//! Redux-like state container for the terminal UI.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, RwLock},
};

use color_eyre::eyre::{Result, eyre};

use crate::{
    config::{Config, ConfigManager},
    ui::{
        colors::{Colors, Theme},
        store::{
            action::Action,
            effect::Effect,
            state::{MAX_LOGS, State},
        },
    },
};

pub mod action;
pub mod derived;
pub mod effect;
pub mod reducer;
pub mod state;

/// Gets application state
pub trait StateGetter {
    fn get_state(&self) -> Result<State>;
}

/// Dispatches actions to update application state
pub trait Dispatcher {
    fn dispatch(&self, action: Action) -> Result<()>;
}

/// Centralized state container with thread-safe access.
pub struct Store {
    state: RwLock<State>,
    reducer: reducer::Reducer,
    config_manager: Arc<Mutex<ConfigManager>>,
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

        let colors = Colors::new(
            theme.to_palette(true_color_enabled),
            true_color_enabled,
        );

        Self {
            reducer: reducer::Reducer::new(),
            config_manager,
            state: RwLock::new(State {
                true_color_enabled,
                ui_paused: false,
                error: None,
                logs: VecDeque::with_capacity(MAX_LOGS),
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

    /// Executes side effects returned by the reducer.
    fn handle_effect(&self, effect: Effect) -> Result<()> {
        match effect {
            Effect::None => Ok(()),
            Effect::CreateConfig(config) => {
                let mut manager = self.config_manager.lock().map_err(|e| {
                    eyre!("failed to lock config manager: {}", e)
                })?;
                manager
                    .create(&config)
                    .map_err(|e| eyre!("failed to create config: {}", e))
            }
            Effect::SaveConfig(config) => {
                let mut manager = self.config_manager.lock().map_err(|e| {
                    eyre!("failed to lock config manager: {}", e)
                })?;
                manager
                    .update_config(config)
                    .map_err(|e| eyre!("failed to save config: {}", e))
            }
        }
    }

    /// Loads a config by ID from the config manager and updates state.
    /// This is separate from dispatch because it requires reading from the
    /// config manager.
    pub fn load_config(&self, config_id: &str) -> Result<()> {
        let config = {
            let manager = self
                .config_manager
                .lock()
                .map_err(|e| eyre!("failed to lock config manager: {}", e))?;
            manager.get_by_id(config_id)
        };

        if let Some(conf) = config {
            let mut state = self.state.write().map_err(|e| {
                eyre!("failed to get write lock on store state: {}", e)
            })?;
            let theme = Theme::from_string(&conf.theme);
            state.config = conf;
            state.colors = Colors::new(
                theme.to_palette(state.true_color_enabled),
                state.true_color_enabled,
            );
        }

        Ok(())
    }
}

impl StateGetter for Store {
    fn get_state(&self) -> Result<State> {
        let state = self.state.read().map_err(|e| {
            eyre!("failed to get read lock on store state: {}", e)
        })?;
        Ok(state.clone())
    }
}

impl Dispatcher for Store {
    fn dispatch(&self, action: action::Action) -> Result<()> {
        let effect = {
            let mut state = self.state.write().map_err(|e| {
                eyre!("failed to get write lock on store state: {}", e)
            })?;
            self.reducer.reduce(&mut state, action)
        }; // Lock released here before I/O

        self.handle_effect(effect)
    }
}

#[cfg(test)]
#[path = "./store_tests.rs"]
mod tests;
