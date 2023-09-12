pub mod editor_popups;
mod generics;
pub mod tree_popups;
use std::io::Stdout;

use crate::configs::PopupMessage;
use crossterm::event::KeyEvent;
pub use generics::message;
pub use generics::{Button, Popup, PopupSelector};
use tui::{backend::CrosstermBackend, Frame};

pub trait PopupInterface {
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>);
    fn map(&mut self, key: &KeyEvent) -> PopupMessage;
}
