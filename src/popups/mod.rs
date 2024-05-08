mod generics;
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
    fn render(&mut self, gs: &mut GlobalState) -> std::io::Result<()>;
    fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        match key {
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Esc, .. } => PopupMessage::Clear,
            _ => self.key_map(key, clipboard),
        }
    }
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage;
    fn update_workspace(&mut self, _workspace: &mut Workspace) {}
    fn update_tree(&mut self, _file_tree: &mut Tree) {}
}

// syntactic sugar for popups used instead of Option<popup>
pub struct PlaceHolderPopup();

impl PopupInterface for PlaceHolderPopup {
    fn key_map(&mut self, _key: &KeyEvent, _clipboard: &mut Clipboard) -> PopupMessage {
        PopupMessage::Clear
    }
    fn render(&mut self, _gs: &mut GlobalState) -> std::io::Result<()> {
        Ok(())
    }
}
