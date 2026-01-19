use crate::ui::{
    colors::{Colors, Theme},
    store::state::{State, ViewID},
};

pub fn set_ui_paused(state: &mut State, value: bool) {
    state.ui_paused = value;
}

pub fn set_error(state: &mut State, err: Option<String>) {
    state.error = err;
}

pub fn toggle_view_select(state: &mut State) {
    state.render_view_select = !state.render_view_select;
}

pub fn update_view(state: &mut State, id: ViewID) {
    state.view_id = id;
}

pub fn update_message(state: &mut State, message: Option<String>) {
    state.message = message;
}

pub fn preview_theme(state: &mut State, theme: Theme) {
    state.colors = Colors::new(
        theme.to_palette(state.true_color_enabled),
        state.true_color_enabled,
    );
}
