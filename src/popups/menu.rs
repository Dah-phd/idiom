use crate::configs::{EditorAction, TreeAction};
use crate::global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage};
use crate::popups::{Command, CommandResult, PopupInterface};
use crate::render::backend::BackendProtocol;
use crate::render::layout::Rect;
use crate::render::state::State;
use crate::tree::Tree;
use crate::workspace::{CursorPosition, Workspace};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use fuzzy_matcher::skim::SkimMatcherV2;

// Go to Definition
// Find References
// Details / Info
// Rename
// Cut
// Copy
// Paste
pub fn menu_context_editor(
    position: CursorPosition,
    screen: Rect,
    accent_style: ContentStyle,
) -> Box<ContextMenuTree<7>> {
    let row_offset = position.line as u16;
    let col_offset = position.char as u16;
    let modal_screen = screen.modal_relative(row_offset, col_offset, 30, 7);

    let menu = ContextMenuTree {
        commands: [
            Command::pass_event("Go to Definition", IdiomEvent::EditorActionCallOnce(EditorAction::GoToDeclaration)),
            Command::pass_event("Find References", IdiomEvent::EditorActionCallOnce(EditorAction::FindReferences)),
            Command::pass_event("Details / Info", IdiomEvent::EditorActionCallOnce(EditorAction::Help)),
            Command::pass_event("Rename", IdiomEvent::EditorActionCallOnce(EditorAction::LSPRename)),
            Command::pass_event("Cut", IdiomEvent::EditorActionCallOnce(EditorAction::Cut)),
            Command::pass_event("Copy", IdiomEvent::EditorActionCallOnce(EditorAction::Copy)),
            Command::pass_event("Paste", IdiomEvent::EditorActionCallOnce(EditorAction::Paste)),
        ],
        modal_screen,
        access_cb: None,
        accent_style,
        rendered: true,
        state: State::new(),
    };
    Box::new(menu)
}

pub fn menu_context_tree(
    position: CursorPosition,
    screen: Rect,
    accent_style: ContentStyle,
) -> Box<ContextMenuTree<7>> {
    let row_offset = position.line as u16;
    let col_offset = position.char as u16;
    let modal_screen = screen.modal_relative(row_offset, col_offset, 30, 7);

    let menu = ContextMenuTree {
        commands: [
            Command::pass_event("Cut", IdiomEvent::TreeActionCallOnce(TreeAction::CutFile)),
            Command::pass_event("Copy", IdiomEvent::TreeActionCallOnce(TreeAction::CopyFile)),
            Command::pass_event("Paste", IdiomEvent::TreeActionCallOnce(TreeAction::Paste)),
            Command::pass_event("Copy Path", IdiomEvent::TreeActionCallOnce(TreeAction::CopyPath)),
            Command::pass_event("Copy Relative Path", IdiomEvent::TreeActionCallOnce(TreeAction::CopyPathRelative)),
            Command::pass_event("Rename", IdiomEvent::TreeActionCallOnce(TreeAction::Rename)),
            Command::pass_event("Delete", IdiomEvent::TreeActionCallOnce(TreeAction::Delete)),
        ],
        modal_screen,
        access_cb: None,
        accent_style,
        rendered: true,
        state: State::with_highlight(ContentStyle::default()),
    };
    Box::new(menu)
}

pub struct ContextMenuTree<const N: usize> {
    commands: [Command; N],
    modal_screen: Rect,
    access_cb: Option<fn(&mut Workspace, &mut Tree)>,
    accent_style: ContentStyle,
    rendered: bool,
    state: State,
}

impl<const N: usize> PopupInterface for ContextMenuTree<N> {
    fn render(&mut self, gs: &mut GlobalState) {
        let reset_style = gs.backend().get_style();
        gs.backend().set_style(self.accent_style);
        self.state.render_list_padded(
            self.commands.iter().map(|c| c.label),
            self.modal_screen.iter_padded(1),
            gs.backend(),
        );
        gs.backend().set_style(reset_style);
    }

    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        match key {
            KeyEvent { code: KeyCode::Up, .. } => self.state.prev(N),
            KeyEvent { code: KeyCode::Down, .. } => self.state.next(N),
            KeyEvent { code: KeyCode::Enter, .. } => {
                return match self.commands[self.state.selected].clone_executor() {
                    CommandResult::Complex(cb) => {
                        self.access_cb.replace(cb);
                        PopupMessage::Event(IdiomEvent::PopupAccessOnce)
                    }
                    CommandResult::Simple(event) => event,
                };
            }
            _ => {}
        }
        PopupMessage::None
    }

    // TODO refactor
    fn mouse_map(&mut self, event: MouseEvent) -> PopupMessage {
        match event.kind {
            MouseEventKind::Moved => {
                let Some(position) = self.modal_screen.relative_position(event.row, event.column) else {
                    return PopupMessage::None;
                };
                if N > position.line {
                    self.mark_as_updated();
                    self.state.selected = position.line;
                };
                PopupMessage::None
            }
            MouseEventKind::Down(MouseButton::Left | MouseButton::Right) => {
                match self.modal_screen.relative_position(event.row, event.column) {
                    None => return PopupMessage::Clear,
                    Some(position) => self.state.selected = position.line,
                }
                match self.commands[self.state.selected].clone_executor() {
                    CommandResult::Complex(cb) => {
                        self.access_cb.replace(cb);
                        PopupMessage::Event(IdiomEvent::PopupAccessOnce)
                    }
                    CommandResult::Simple(event) => event,
                }
            }
            _ => PopupMessage::None,
        }
    }

    fn component_access(&mut self, ws: &mut Workspace, tree: &mut Tree) {
        if let Some(cb) = self.access_cb {
            (cb)(ws, tree);
        }
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.rendered)
    }

    fn mark_as_updated(&mut self) {
        self.rendered = true
    }
}
