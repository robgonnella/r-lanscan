//! UI state reducers for pausing, errors, views, and themes.

use crate::ui::{
    colors::{Colors, Theme},
    store::state::{State, ViewID},
};

/// Sets whether the UI is paused (for shell command execution).
pub fn set_ui_paused(state: &mut State, value: bool) {
    state.ui_paused = value;
}

/// Sets or clears the current error message.
pub fn set_error(state: &mut State, err: Option<String>) {
    state.error = err;
}

/// Toggles the view selection menu visibility.
pub fn toggle_view_select(state: &mut State) {
    state.render_view_select = !state.render_view_select;
}

/// Changes the active view to the specified ID.
pub fn update_view(state: &mut State, id: ViewID) {
    state.view_id = id;
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
