mod generics;
pub mod menu;
pub mod pallet;
pub mod popup_file_open;
pub mod popup_find;
pub mod popup_replace;
pub mod popup_tree_search;
pub mod popups_editor;
pub mod popups_tree;
pub mod tui_embed;
mod utils;

use crate::{
    configs::CONFIG_FOLDER,
    global_state::{Clipboard, IdiomEvent, PopupMessage},
    render::{backend::Backend, layout::Rect},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use dirs::config_dir;
use fuzzy_matcher::skim::SkimMatcherV2;
pub use generics::{Popup, PopupSelector};

pub const NULL_POPUP: PlaceHolderPopup = PlaceHolderPopup();

pub fn placeholder() -> Box<PlaceHolderPopup> {
    Box::new(NULL_POPUP)
}

pub trait PopupInterface {
    fn fast_render(&mut self, screen: Rect, backend: &mut Backend) {
        if self.collect_update_status() {
            self.render(screen, backend);
        }
    }

    fn mouse_map(&mut self, _event: MouseEvent) -> PopupMessage {
        PopupMessage::None
    }

    fn map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage {
        self.mark_as_updated();
        match key {
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => PopupMessage::Clear,
            KeyEvent { code: KeyCode::Esc, .. } => PopupMessage::Clear,
            _ => self.key_map(key, clipboard, matcher),
        }
    }

    fn render(&mut self, screen: Rect, backend: &mut Backend);
    fn resize(&mut self, new_screen: Rect) -> PopupMessage;
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage;
    fn component_access(&mut self, _ws: &mut Workspace, _tree: &mut Tree) {}
    fn mark_as_updated(&mut self);
    fn collect_update_status(&mut self) -> bool;
    fn paste_passthrough(&mut self, _clip: String, _matcher: &SkimMatcherV2) -> PopupMessage {
        PopupMessage::None
    }
}

struct Command {
    label: &'static str,
    result: CommandResult,
}

impl Command {
    fn execute(self) -> CommandResult {
        self.result
    }

    fn clone_executor(&self) -> CommandResult {
        self.result.clone()
    }

    fn cfg_open(label: &'static str, file_path: &'static str) -> Option<Self> {
        let mut path = config_dir()?;
        path.push(CONFIG_FOLDER);
        path.push(file_path);
        Some(Command { label, result: CommandResult::Simple(IdiomEvent::OpenAtLine(path, 0).into()) })
    }

    fn pass_event(label: &'static str, event: IdiomEvent) -> Self {
        Command { label, result: CommandResult::Simple(event.into()) }
    }

    const fn access_edit(label: &'static str, cb: fn(&mut Workspace, &mut Tree)) -> Self {
        Command { label, result: CommandResult::Complex(cb) }
    }
}

#[derive(Debug, Clone)]
enum CommandResult {
    Simple(PopupMessage),
    Complex(fn(&mut Workspace, &mut Tree)),
}

// syntactic sugar for popups used instead of Option<popup>
pub struct PlaceHolderPopup();

impl PopupInterface for PlaceHolderPopup {
    fn key_map(&mut self, _key: &KeyEvent, _clipboard: &mut Clipboard, _matcher: &SkimMatcherV2) -> PopupMessage {
        PopupMessage::Clear
    }

    fn mark_as_updated(&mut self) {}
    fn collect_update_status(&mut self) -> bool {
        false
    }
    fn render(&mut self, _rect: Rect, _backend: &mut Backend) {}
    fn resize(&mut self, _new_screen: Rect) -> PopupMessage {
        PopupMessage::None
    }
}
