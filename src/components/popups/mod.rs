pub mod editor_popups;
mod generics;
pub mod tree_popups;

use crate::events::messages::PopupMessage;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub use generics::message;
pub use generics::{Button, Popup, PopupSelector};
use ratatui::Frame;

use super::{Tree, Workspace};

pub trait PopupInterface {
    fn render(&mut self, frame: &mut Frame);
    fn map(&mut self, key: &KeyEvent) -> PopupMessage {
        match key {
            KeyEvent { code: KeyCode::Char('d') | KeyCode::Char('D'), modifiers: KeyModifiers::CONTROL, .. } => {
                return PopupMessage::Done
            }
            KeyEvent { code: KeyCode::Char('q') | KeyCode::Char('Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                return PopupMessage::Done
            }
            _ => (),
        }
        self.key_map(key)
    }
    fn key_map(&mut self, key: &KeyEvent) -> PopupMessage;
    fn update_workspace(&mut self, workspace: &mut Workspace);
    fn update_tree(&mut self, file_tree: &mut Tree);
}
