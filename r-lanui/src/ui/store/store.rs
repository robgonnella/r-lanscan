use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::config::{ConfigManager, DEFAULT_CONFIG_ID};

use super::{
    action::Action,
    reducer::Reducer,
    state::{Colors, State, Theme, ViewID},
};

/**
 * Manages the state of our application
 */
pub struct Store {
    state: Mutex<State>,
    reducer: Reducer,
}

impl Store {
    pub fn new<'conf_man>(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        let config = config_manager
            .lock()
            .unwrap()
            .get_by_id(&DEFAULT_CONFIG_ID.to_string())
            .unwrap();

        let theme = Theme::from_string(&config.theme);
        let colors = Colors::new(theme.to_palette());

        Self {
            reducer: Reducer::new(config_manager),
            state: Mutex::new(State {
                error: None,
                render_view_select: false,
                view_id: ViewID::Devices,
                config,
                devices: Vec::new(),
                device_map: HashMap::new(),
                selected_device: None,
                selected_device_config: None,
                colors,
                message: None,
                execute_cmd: None,
                cmd_output: None,
                cmd_in_progress: false,
            }),
        }
    }

    pub fn dispatch(&self, action: Action) {
        let mut prev_state = self.state.lock().unwrap();
        let new_state = self.reducer.reduce(prev_state.clone(), action);
        *prev_state = new_state;
    }

    pub fn get_state(&self) -> State {
        self.state.lock().unwrap().clone()
    }
}
