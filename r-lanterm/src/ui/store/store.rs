use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    config::{ConfigManager, DEFAULT_CONFIG_ID},
    ui::colors::Theme,
};

use super::{
    action::Action,
    reducer::Reducer,
    state::{State, ViewID},
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

        let true_color_enabled = match supports_color::on(supports_color::Stream::Stdout) {
            Some(support) => support.has_16m,
            _ => false,
        };

        let theme = Theme::from_string(&config.theme);
        let colors = crate::ui::colors::Colors::new(
            theme.to_palette(true_color_enabled),
            true_color_enabled,
        );

        Self {
            reducer: Reducer::new(config_manager),
            state: Mutex::new(State {
                true_color_enabled,
                ui_paused: false,
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
                cmd_in_progress: None,
                cmd_output: None,
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

#[cfg(test)]
mod tests {
    use nanoid::nanoid;
    use std::fs;

    use super::*;

    fn setup() -> (Store, String) {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));
        let store = Store::new(conf_manager);
        (store, tmp_path)
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_new() {
        let (store, conf_path) = setup();
        assert!(store.state.lock().is_ok());
        tear_down(conf_path);
    }
}
