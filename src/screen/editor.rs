use crate::state::State;
use std::sync::{Arc, RwLock};
use tui::{backend::Backend, Frame};

pub fn editor(terminal: &mut Frame<impl Backend>, state: Arc<RwLock<State>>) {
    if state.read().unwrap().opened_files.editors.is_empty() {
        return;
    }
}
