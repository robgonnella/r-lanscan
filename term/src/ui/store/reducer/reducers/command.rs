use std::process::Output;

use crate::{ipc::message::Command, ui::store::state::State};

pub fn set_command_in_progress(state: &mut State, value: Option<Command>) {
    state.cmd_in_progress = value;
}

pub fn update_command_output(state: &mut State, cmd: Command, output: Output) {
    state.cmd_output = Some((cmd, output));
}

pub fn clear_command_output(state: &mut State) {
    state.cmd_output = None;
}
