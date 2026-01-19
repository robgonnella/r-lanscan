use crate::ui::{
    colors::{Colors, Theme},
    store::state::{State, ViewID},
};

pub fn set_ui_paused(prev_state: State, value: bool) -> State {
    let mut state = prev_state.clone();
    state.ui_paused = value;
    state
}

pub fn set_error(prev_state: State, err: Option<String>) -> State {
    let mut state = prev_state.clone();
    state.error = err;
    state
}

pub fn toggle_view_select(prev_state: State) -> State {
    let mut state = prev_state.clone();
    state.render_view_select = !state.render_view_select;
    state
}

pub fn update_view(prev_state: State, id: ViewID) -> State {
    let mut state = prev_state.clone();
    state.view_id = id;
    state
}

pub fn update_message(prev_state: State, message: Option<String>) -> State {
    let mut state = prev_state.clone();
    state.message = message;
    state
}

pub fn preview_theme(prev_state: State, theme: Theme) -> State {
    let mut state = prev_state.clone();
    state.colors = Colors::new(
        theme.to_palette(state.true_color_enabled),
        state.true_color_enabled,
    );
    state
}
