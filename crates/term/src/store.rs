//! Redux-like state container for the terminal UI.

#[cfg(test)]
use mockall::automock;

use std::{cell::RefCell, rc::Rc};

use crate::store::{action::Action, state::State};

pub mod action;
pub mod reducer;
pub mod state;

/// Gets application state
#[cfg_attr(test, automock)]
pub trait StateGetter {
    fn get_state(&self) -> Rc<State>;
}

/// Dispatches actions to update application state
#[cfg_attr(test, automock)]
pub trait Dispatcher {
    fn dispatch(&self, action: Action);
}

/// Handles mutating store state based on provided action
#[cfg_attr(test, automock)]
pub trait Reducer {
    fn reduce(&self, state: &mut State, action: Action);
}

/// Centralized state container
pub struct Store {
    state: RefCell<Rc<State>>,
    reducer: Box<dyn Reducer>,
    sync: Option<Box<dyn Fn(Action)>>,
}

impl Store {
    /// Creates a new store with the given config manager and initial config.
    pub fn new(initial_state: State, reducer: Box<dyn Reducer>) -> Self {
        Self {
            reducer,
            state: RefCell::new(Rc::new(initial_state)),
            sync: None,
        }
    }

    pub fn set_sync_fn<F: Fn(Action) + 'static>(&mut self, f: F) {
        self.sync = Some(Box::new(f))
    }
}

impl StateGetter for Store {
    fn get_state(&self) -> Rc<State> {
        self.state.borrow().clone()
    }
}

impl Dispatcher for Store {
    fn dispatch(&self, action: Action) {
        {
            let mut rc = self.state.borrow_mut();
            let state = Rc::make_mut(&mut rc);

            // prevent recursively syncing actions back to sync-er
            if let Action::Sync(a) = action {
                self.reducer.reduce(state, a.as_ref().to_owned());
                return;
            }

            self.reducer.reduce(state, action.clone());
        }

        if let Some(f) = self.sync.as_ref() {
            f(action)
        }
    }
}
