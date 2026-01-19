use std::process::Output;

use crate::{events::types::Command, ui::store::state::State};

pub fn set_command_in_progress(prev_state: State, value: Option<Command>) -> State {
    let mut state = prev_state.clone();
    state.cmd_in_progress = value;
    state
}

pub fn update_command_output(prev_state: State, cmd: Command, output: Output) -> State {
    let mut state = prev_state.clone();
    state.cmd_output = Some((cmd, output));
    state
}

pub fn clear_command_output(prev_state: State) -> State {
    let mut state = prev_state.clone();
    state.cmd_output = None;
    state
}
