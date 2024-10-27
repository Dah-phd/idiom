mod generics;
pub mod pallet;
pub mod popup_find;
pub mod popup_replace;
pub mod popup_tree_search;
pub mod popups_editor;
pub mod popups_tree;
mod utils;

use crate::{
    global_state::{Clipboard, GlobalState, PopupMessage},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub use generics::{Popup, PopupSelector};

pub const NULL_POPUP: PlaceHolderPopup = PlaceHolderPopup();

pub fn placeholder() -> Box<PlaceHolderPopup> {
    Box::new(NULL_POPUP)
}

pub trait PopupInterface {
    fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.collect_update_status() {
            self.render(gs);
        }
    }

    fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        self.mark_as_updated();
        match key {
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Esc, .. } => PopupMessage::Clear,
            _ => self.key_map(key, clipboard),
        }
    }

    fn render(&mut self, gs: &mut GlobalState);
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage;
    fn component_access(&mut self, _ws: &mut Workspace, _tree: &mut Tree) {}
    fn mark_as_updated(&mut self);
    fn collect_update_status(&mut self) -> bool;
}

// syntactic sugar for popups used instead of Option<popup>
pub struct PlaceHolderPopup();

impl PopupInterface for PlaceHolderPopup {
    fn key_map(&mut self, _key: &KeyEvent, _clipboard: &mut Clipboard) -> PopupMessage {
        PopupMessage::Clear
    }

    fn mark_as_updated(&mut self) {}
    fn collect_update_status(&mut self) -> bool {
        false
    }
    fn render(&mut self, _gs: &mut GlobalState) {}
}
