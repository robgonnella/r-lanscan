use std::sync::{Arc, Mutex};

use crate::config::ConfigManager;

use super::{action::Action, state::State};

mod reducers;

pub struct Reducer {
    config_manager: Arc<Mutex<ConfigManager>>,
}

impl Reducer {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self { config_manager }
    }

    pub fn reduce(&self, prev_state: State, action: Action) -> State {
        match action {
            // UI actions
            Action::SetUIPaused(value) => reducers::ui::set_ui_paused(prev_state, value),
            Action::SetError(err) => reducers::ui::set_error(prev_state, err),
            Action::ToggleViewSelect => reducers::ui::toggle_view_select(prev_state),
            Action::UpdateView(id) => reducers::ui::update_view(prev_state, id),
            Action::UpdateMessage(message) => reducers::ui::update_message(prev_state, message),
            Action::PreviewTheme(theme) => reducers::ui::preview_theme(prev_state, theme),

            // Device actions
            Action::UpdateAllDevices(devices) => {
                reducers::device::update_all_devices(prev_state, devices)
            }
            Action::AddDevice(device) => reducers::device::add_device(prev_state, device),
            Action::UpdateSelectedDevice(ip) => {
                reducers::device::update_selected_device(prev_state, ip)
            }

            // Config actions
            Action::UpdateConfig(config) => {
                reducers::config::update_config(prev_state, config, &self.config_manager)
            }
            Action::SetConfig(config_id) => {
                reducers::config::set_config(prev_state, config_id, &self.config_manager)
            }
            Action::CreateAndSetConfig(config) => {
                reducers::config::create_and_set_config(prev_state, config, &self.config_manager)
            }
            Action::UpdateDeviceConfig(device_config) => reducers::config::update_device_config(
                prev_state,
                device_config,
                &self.config_manager,
            ),

            // Command actions
            Action::SetCommandInProgress(value) => {
                reducers::command::set_command_in_progress(prev_state, value)
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                reducers::command::update_command_output(prev_state, cmd, output)
            }
            Action::ClearCommandOutput => reducers::command::clear_command_output(prev_state),
        }
    }
}

#[cfg(test)]
#[path = "./reducer_tests.rs"]
mod tests;
