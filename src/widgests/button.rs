use crate::{global_state::PopupMessage, popups::Popup};
use crossterm::event::KeyCode;

#[derive(Clone)]
pub struct Button {
    pub command: fn(&mut Popup) -> PopupMessage,
    pub name: &'static str,
    pub key: Option<Vec<KeyCode>>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("").field(&self.name).finish()
    }
}
