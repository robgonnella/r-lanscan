//! Command state reducers for tracking shell command execution.

use std::process::Output;

use crate::{ipc::message::Command, ui::store::state::State};

/// Sets or clears the currently executing command.
pub fn set_command_in_progress(state: &mut State, value: Option<Command>) {
    if let Some(cmd) = value.as_ref() {
        state.popover_message = Some(format!("Executing command: {cmd}"));
    } else {
        state.popover_message = None
    }
    state.cmd_in_progress = value;
}

/// Stores the output from a completed command.
pub fn update_command_output(state: &mut State, cmd: Command, output: Output) {
    state.cmd_output = Some((cmd, output));
}

/// Clears stored command output.
pub fn clear_command_output(state: &mut State) {
    state.cmd_output = None;
}
