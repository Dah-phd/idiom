use super::{Command, CommandResult, Components, InplacePopup, Status};
use crate::configs::{EditorAction, TreeAction};
use crate::global_state::{GlobalState, IdiomEvent};
use crate::render::backend::BackendProtocol;
use crate::render::layout::Rect;
use crate::render::state::State;
use crate::workspace::CursorPosition;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;

pub fn menu_context_editor_inplace(
    position: CursorPosition,
    screen: Rect,
    accent_style: ContentStyle,
) -> ContextMenu<7> {
    let row_offset = position.line as u16;
    let col_offset = position.char as u16;
    let modal_screen = screen.modal_relative(row_offset, col_offset, 30, 7);

    ContextMenu {
        commands: [
            Command::pass_event("Go to Definition", IdiomEvent::EditorActionCall(EditorAction::GoToDeclaration)),
            Command::pass_event("Find References", IdiomEvent::EditorActionCall(EditorAction::FindReferences)),
            Command::pass_event("Details / Info", IdiomEvent::EditorActionCall(EditorAction::Help)),
            Command::pass_event("Rename", IdiomEvent::EditorActionCall(EditorAction::LSPRename)),
            Command::pass_event("Cut", IdiomEvent::EditorActionCall(EditorAction::Cut)),
            Command::pass_event("Copy", IdiomEvent::EditorActionCall(EditorAction::Copy)),
            Command::pass_event("Paste", IdiomEvent::EditorActionCall(EditorAction::Paste)),
        ],
        modal_screen,
        accent_style,
        state: State::new(),
    }
}

pub fn menu_context_tree_inplace(position: CursorPosition, screen: Rect, accent_style: ContentStyle) -> ContextMenu<7> {
    let row_offset = position.line as u16;
    let col_offset = position.char as u16;
    let modal_screen = screen.modal_relative(row_offset, col_offset, 30, 7);

    ContextMenu {
        commands: [
            Command::pass_event("Cut", IdiomEvent::TreeActionCall(TreeAction::CutFile)),
            Command::pass_event("Copy", IdiomEvent::TreeActionCall(TreeAction::CopyFile)),
            Command::pass_event("Paste", IdiomEvent::TreeActionCall(TreeAction::Paste)),
            Command::pass_event("Copy Path", IdiomEvent::TreeActionCall(TreeAction::CopyPath)),
            Command::pass_event("Copy Relative Path", IdiomEvent::TreeActionCall(TreeAction::CopyPathRelative)),
            Command::pass_event("Rename", IdiomEvent::TreeActionCall(TreeAction::Rename)),
            Command::pass_event("Delete", IdiomEvent::TreeActionCall(TreeAction::Delete)),
        ],
        modal_screen,
        accent_style,
        state: State::with_highlight(ContentStyle::default()),
    }
}

pub struct ContextMenu<const N: usize> {
    commands: [Command; N],
    modal_screen: Rect,
    accent_style: ContentStyle,
    state: State,
}

impl<const N: usize> InplacePopup for ContextMenu<N> {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let backend = gs.backend();
        let reset_style = backend.get_style();
        backend.set_style(self.accent_style);
        self.state.render_list_padded(self.commands.iter().map(|c| c.label), self.modal_screen.iter_padded(1), backend);
        backend.set_style(reset_style);
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut super::Components) -> Status<Self::R> {
        let Components { gs, ws, tree, .. } = components;
        match key {
            KeyEvent { code: KeyCode::Up, .. } => self.state.prev(N),
            KeyEvent { code: KeyCode::Down, .. } => self.state.next(N),
            KeyEvent { code: KeyCode::Enter, .. } => {
                match self.commands[self.state.selected].clone_executor() {
                    CommandResult::Complex(cb) => {
                        cb(ws, tree);
                    }
                    CommandResult::Simple(event) => gs.event.push(event),
                };
                return Status::Dropped;
            }
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut super::Components) -> Status<Self::R> {
        let Components { gs, ws, tree, .. } = components;
        match event.kind {
            MouseEventKind::Moved => {
                if let Some(position) = self.modal_screen.relative_position(event.row, event.column) {
                    if N > position.line {
                        self.state.selected = position.line;
                        self.force_render(gs);
                    };
                };
            }
            MouseEventKind::Down(MouseButton::Left | MouseButton::Right) => {
                if let Some(position) = self.modal_screen.relative_position(event.row, event.column) {
                    self.state.selected = position.line;
                    match self.commands[self.state.selected].clone_executor() {
                        CommandResult::Complex(cb) => cb(ws, tree),
                        CommandResult::Simple(event) => gs.event.push(event),
                    }
                }
                return Status::Dropped;
            }
            _ => (),
        }
        Status::Pending
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        false
    }

    fn paste_passthrough(&mut self, _clip: String, _components: &mut super::Components) -> bool {
        false
    }

    fn render(&mut self, _: &mut GlobalState) {}
}
