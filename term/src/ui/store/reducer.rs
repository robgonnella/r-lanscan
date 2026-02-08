//! Pure reducer functions that compute new state from actions.

use std::fmt::Debug;

use super::{action::Action, effect::Effect, state::State};

mod reducers;

/// Applies actions to state, producing new state and optional side effects.
#[derive(Default)]
pub struct Reducer;

impl Reducer {
    /// Creates a new reducer.
    pub fn new() -> Self {
        Self
    }

    /// Applies an action to the state, mutating it in place and returning any
    /// side effects to be executed.
    pub fn reduce(&self, state: &mut State, action: Action) -> Effect {
        match action {
            // UI actions
            Action::SetUIPaused(value) => {
                self.log_action("SetUIPaused", &value, state);
                reducers::ui::set_ui_paused(state, value);
                Effect::None
            }
            Action::SetError(err) => {
                self.log_action("SetError", &err, state);
                reducers::ui::set_error(state, err);
                Effect::None
            }
            Action::Log(log) => {
                log::debug!("{log}");
                if state.logs.len() == state.logs.capacity() {
                    state.logs.pop_front();
                }
                state.logs.push_back(log);
                Effect::None
            }
            Action::UpdateMessage(message) => {
                self.log_action("UpdateMessage", &message, state);
                reducers::ui::update_message(state, message);
                Effect::None
            }
            Action::PreviewTheme(theme) => {
                self.log_action("PreviewTheme", &theme, state);
                reducers::ui::preview_theme(state, theme);
                Effect::None
            }

            // Device actions
            Action::UpdateAllDevices(devices) => {
                self.log_action("UpdateAllDevices", &devices, state);
                reducers::device::update_all_devices(state, devices);
                Effect::None
            }
            Action::AddDevice(device) => {
                self.log_action("AddDevice", &device, state);
                reducers::device::add_device(state, device);
                Effect::None
            }

            // Config actions
            Action::UpdateConfig(config) => {
                self.log_action("UpdateConfig", &config, state);
                reducers::config::update_config(state, &config);
                Effect::SaveConfig(config)
            }
            Action::CreateAndSetConfig(config) => {
                self.log_action("CreateAndSetConfig", &config, state);
                reducers::config::create_and_set_config(state, &config);
                Effect::CreateConfig(config)
            }
            Action::UpdateDeviceConfig(device_config) => {
                self.log_action("UpdateDeviceConfig", &device_config, state);
                let config = reducers::config::update_device_config(
                    state,
                    device_config,
                );
                Effect::SaveConfig(config)
            }

            // Command actions
            Action::SetCommandInProgress(value) => {
                self.log_action("SetCommandInProgress", &value, state);
                reducers::command::set_command_in_progress(state, value);
                Effect::None
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                self.log_action("UpdateCommandOutput", &output, state);
                reducers::command::update_command_output(state, cmd, output);
                Effect::None
            }
            Action::ClearCommandOutput => {
                self.log_action("ClearCommandOutput", &"", state);
                reducers::command::clear_command_output(state);
                Effect::None
            }
        }
    }

    fn log_action<D: Debug>(&self, name: &str, data: &D, state: &mut State) {
        state
            .logs
            .push_back(format!("processing action: {name}({:?})", data));
    }
}

#[cfg(test)]
#[path = "./reducer_tests.rs"]
mod tests;
