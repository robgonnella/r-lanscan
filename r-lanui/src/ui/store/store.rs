use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use r_lanlib::scanners::Device;

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
                arp_history: HashMap::new(),
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

    // returns just the devices that were detected in last arp scan
    // i.e. miss count = 0
    pub fn get_detected_devices(&self) -> Vec<Device> {
        let locked = self.state.lock().unwrap();
        locked
            .arp_history
            .iter()
            .filter(|d| d.1 .1 == 0)
            .map(|d| d.1 .0.clone())
            .collect::<Vec<Device>>()
    }
}
