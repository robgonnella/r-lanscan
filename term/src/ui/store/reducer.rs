//! Pure reducer functions that compute new state from actions.

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
                reducers::ui::set_ui_paused(state, value);
                Effect::None
            }
            Action::SetError(err) => {
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
            Action::ToggleViewSelect => {
                reducers::ui::toggle_view_select(state);
                Effect::None
            }
            Action::UpdateView(id) => {
                reducers::ui::update_view(state, id);
                Effect::None
            }
            Action::UpdateMessage(message) => {
                reducers::ui::update_message(state, message);
                Effect::None
            }
            Action::PreviewTheme(theme) => {
                reducers::ui::preview_theme(state, theme);
                Effect::None
            }

            // Device actions
            Action::UpdateAllDevices(devices) => {
                reducers::device::update_all_devices(state, devices);
                Effect::None
            }
            Action::AddDevice(device) => {
                reducers::device::add_device(state, device);
                Effect::None
            }
            Action::UpdateSelectedDevice(ip) => {
                reducers::device::update_selected_device(state, ip);
                Effect::None
            }

            // Config actions
            Action::UpdateConfig(config) => {
                reducers::config::update_config(state, &config);
                Effect::SaveConfig(config)
            }
            Action::CreateAndSetConfig(config) => {
                reducers::config::create_and_set_config(state, &config);
                Effect::CreateConfig(config)
            }
            Action::UpdateDeviceConfig(device_config) => {
                let config = reducers::config::update_device_config(
                    state,
                    device_config,
                );
                Effect::SaveConfig(config)
            }

            // Command actions
            Action::SetCommandInProgress(value) => {
                reducers::command::set_command_in_progress(state, value);
                Effect::None
            }
            Action::UpdateCommandOutput((cmd, output)) => {
                reducers::command::update_command_output(state, cmd, output);
                Effect::None
            }
            Action::ClearCommandOutput => {
                reducers::command::clear_command_output(state);
                Effect::None
            }
        }
    }
}

#[cfg(test)]
#[path = "./reducer_tests.rs"]
mod tests;
