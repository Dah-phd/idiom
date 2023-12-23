mod generics;
pub mod popup_find;
pub mod popup_replace;
pub mod popup_tree_search;
pub mod popups_editor;
pub mod popups_tree;
mod utils;

use crate::global_state::{Clipboard, PopupMessage};
use crate::{tree::Tree, workspace::Workspace};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub use generics::{Button, Popup, PopupSelector};
use ratatui::Frame;

pub trait PopupInterface {
    fn render(&mut self, frame: &mut Frame);
    fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        match key {
            KeyEvent { code: KeyCode::Char('d') | KeyCode::Char('D'), modifiers: KeyModifiers::CONTROL, .. } => {
                return PopupMessage::Clear
            }
            KeyEvent { code: KeyCode::Char('q') | KeyCode::Char('Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                return PopupMessage::Clear
            }
            _ => (),
        }
        self.key_map(key, clipboard)
    }
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage;
    fn update_workspace(&mut self, workspace: &mut Workspace);
    fn update_tree(&mut self, file_tree: &mut Tree);
}
