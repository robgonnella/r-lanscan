use std::sync::{Arc, Mutex};

use crate::config::ConfigManager;

use super::{action::Action, state::State, store::Store};

pub struct Dispatcher {
    store: Mutex<Store>,
}

impl Dispatcher {
    pub fn new<'conf_man>(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self {
            store: Mutex::new(Store::new(config_manager)),
        }
    }

    pub fn dispatch(&self, action: Action) {
        let mut store = self.store.lock().unwrap();
        store.update(action);
    }

    pub fn get_state(&self) -> State {
        let store = self.store.lock().unwrap();
        store.get_state()
    }
}
