//! UI state reducers for pausing, errors, views, and themes.

use crate::ui::{
    colors::{Colors, Theme},
    store::state::State,
};

/// Sets whether the UI is paused (for shell command execution).
pub fn set_ui_paused(state: &mut State, value: bool) {
    state.ui_paused = value;
}

/// Sets or clears the current error message.
pub fn set_error(state: &mut State, err: Option<String>) {
    state.error = err;
}

/// Sets or clears a status message (e.g., scan progress).
pub fn update_message(state: &mut State, message: Option<String>) {
    state.message = message;
}

/// Applies a theme preview without persisting to config.
pub fn preview_theme(state: &mut State, theme: Theme) {
    state.colors = Colors::new(
        theme.to_palette(state.true_color_enabled),
        state.true_color_enabled,
    );
}
