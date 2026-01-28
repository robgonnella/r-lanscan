//! Pure reducer functions that compute new state from actions.

use std::sync::{Arc, Mutex};

use crate::config::ConfigManager;

use super::{action::Action, state::State};

mod reducers;

/// Applies actions to state, producing new state.
pub struct Reducer {
    config_manager: Arc<Mutex<ConfigManager>>,
}

impl Reducer {
    /// Creates a new reducer with the given config manager for persistence.
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self { config_manager }
    }

    /// Applies an action to the state, mutating it in place.
    pub fn reduce(&self, state: &mut State, action: Action) {
        match action {
            // UI actions
            Action::SetUIPaused(value) => {
                reducers::ui::set_ui_paused(state, value)
            }
            Action::SetError(err) => reducers::ui::set_error(state, err),
            Action::Log(log) => {
                log::debug!("{log}");
                if state.logs.len() == state.logs.capacity() {
                    state.logs.pop_front();
                }
                state.logs.push_back(log);
            }
            Action::ToggleViewSelect => reducers::ui::toggle_view_select(state),
            Action::UpdateView(id) => reducers::ui::update_view(state, id),
            Action::UpdateMessage(message) => {
                reducers::ui::update_message(state, message)
            }
            Action::PreviewTheme(theme) => {
                reducers::ui::preview_theme(state, theme)
            }

            // Device actions
            Action::UpdateAllDevices(devices) => {
                reducers::device::update_all_devices(state, devices)
            }
            Action::AddDevice(device) => {
                reducers::device::add_device(state, device)
            }
            Action::UpdateSelectedDevice(ip) => {
                reducers::device::update_selected_device(state, ip)
            }

            // Config actions
            Action::UpdateConfig(config) => reducers::config::update_config(
                state,
                config,
                &self.config_manager,
            ),
            Action::SetConfig(config_id) => reducers::config::set_config(
                state,
                config_id,
                &self.config_manager,
            ),
            Action::CreateAndSetConfig(config) => {
                reducers::config::create_and_set_config(
                    state,
                    config,
                    &self.config_manager,
                )
            }
            Action::UpdateDeviceConfig(device_config) => {
                reducers::config::update_device_config(
                    state,
                    device_config,
                    &self.config_manager,
                )
            }

            // Command actions
            Action::SetCommandInProgress(value) => {
                reducers::command::set_command_in_progress(state, value)
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                reducers::command::update_command_output(state, cmd, output)
            }
            Action::ClearCommandOutput => {
                reducers::command::clear_command_output(state)
            }
        }
    }
}

#[cfg(test)]
#[path = "./reducer_tests.rs"]
mod tests;
