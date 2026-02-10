//! Pure reducer functions that compute new state from actions.

use std::fmt::Debug;

use crate::store::Reducer;

use super::{action::Action, state::State};

mod reducers;

/// Applies actions to state, producing new state and optional side effects.
#[derive(Default)]
pub struct StoreReducer;

impl StoreReducer {
    pub fn boxed() -> Box<Self> {
        Box::default()
    }

    fn log_action<D: Debug>(&self, name: &str, data: &D, state: &mut State) {
        state
            .logs
            .push_back(format!("processing action: {name}({:?})", data));
    }
}

impl Reducer for StoreReducer {
    /// Applies an action to the state, mutating it in place and returning any
    /// side effects to be executed.
    fn reduce(&self, state: &mut State, action: Action) {
        match action {
            // UI actions
            Action::SetUIPaused(value) => {
                self.log_action("SetUIPaused", &value, state);
                reducers::ui::set_ui_paused(state, value);
            }
            Action::SetError(err) => {
                self.log_action("SetError", &err, state);
                reducers::ui::set_error(state, err);
            }
            Action::Log(log) => {
                log::debug!("{log}");
                if state.logs.len() == state.logs.capacity() {
                    state.logs.pop_front();
                }
                state.logs.push_back(log);
            }
            Action::UpdateMessage(message) => {
                self.log_action("UpdateMessage", &message, state);
                reducers::ui::update_message(state, message);
            }
            Action::PreviewTheme(theme) => {
                self.log_action("PreviewTheme", &theme, state);
                reducers::ui::preview_theme(state, theme);
            }

            // Device actions
            Action::UpdateAllDevices(devices) => {
                self.log_action("UpdateAllDevices", &devices, state);
                reducers::device::update_all_devices(state, devices);
            }
            Action::AddDevice(device) => {
                self.log_action("AddDevice", &device, state);
                reducers::device::add_device(state, device);
            }

            // Config actions
            Action::UpdateConfig(config) => {
                self.log_action("UpdateConfig", &config, state);
                reducers::config::update_config(state, &config);
            }
            Action::UpdateDeviceConfig(device_config) => {
                self.log_action("UpdateDeviceConfig", &device_config, state);
                reducers::config::update_device_config(state, device_config);
            }

            // Command actions
            Action::SetCommandInProgress(value) => {
                self.log_action("SetCommandInProgress", &value, state);
                reducers::command::set_command_in_progress(state, value);
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                self.log_action("UpdateCommandOutput", &output, state);
                reducers::command::update_command_output(state, cmd, output);
            }
            Action::ClearCommandOutput => {
                self.log_action("ClearCommandOutput", &"", state);
                reducers::command::clear_command_output(state);
            }
            Action::Sync(_) => {}
        }
    }
}

#[cfg(test)]
#[path = "./reducer_tests.rs"]
mod tests;
