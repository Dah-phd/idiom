use super::{Components, Popup, Status};
use crate::configs::{EditorAction, TreeAction};
use crate::global_state::GlobalState;
use crate::render::backend::BackendProtocol;
use crate::render::layout::Rect;
use crate::render::state::State;
use crate::workspace::CursorPosition;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;

enum Action {
    Tree(TreeAction),
    Editor(EditorAction),
}

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
            ("Go to Definition", EditorAction::GoToDeclaration.into()),
            ("Find References", EditorAction::FindReferences.into()),
            ("Details / Info", EditorAction::Help.into()),
            ("Rename", EditorAction::LSPRename.into()),
            ("Cut", EditorAction::Cut.into()),
            ("Copy", EditorAction::Copy.into()),
            ("Paste", EditorAction::Paste.into()),
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
            ("Cut", TreeAction::CutFile.into()),
            ("Copy", TreeAction::CopyFile.into()),
            ("Paste", TreeAction::Paste.into()),
            ("Copy Path", TreeAction::CopyPath.into()),
            ("Copy Relative Path", TreeAction::CopyPathRelative.into()),
            ("Rename", TreeAction::Rename.into()),
            ("Delete", TreeAction::Delete.into()),
        ],
        modal_screen,
        accent_style,
        state: State::with_highlight(ContentStyle::default()),
    }
}

pub struct ContextMenu<const N: usize> {
    commands: [(&'static str, Action); N],
    modal_screen: Rect,
    accent_style: ContentStyle,
    state: State,
}

impl<const N: usize> Popup for ContextMenu<N> {
    type R = ();

    fn force_render(&mut self, gs: &mut GlobalState) {
        let backend = gs.backend();
        let reset_style = backend.get_style();
        backend.set_style(self.accent_style);
        self.state.render_list_padded(self.commands.iter().map(|c| c.0), self.modal_screen.iter_padded(1), backend);
        backend.set_style(reset_style);
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut super::Components) -> Status<Self::R> {
        let Components { gs, ws, tree, .. } = components;
        match key {
            KeyEvent { code: KeyCode::Up, .. } => self.state.prev(N),
            KeyEvent { code: KeyCode::Down, .. } => self.state.next(N),
            KeyEvent { code: KeyCode::Enter, .. } => {
                match self.commands[self.state.selected].1 {
                    Action::Tree(action) => {
                        tree.map_action(action, gs);
                    }
                    Action::Editor(action) => {
                        if let Some(editor) = ws.get_active() {
                            editor.map(action, gs);
                        }
                    }
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
                    match self.commands[self.state.selected].1 {
                        Action::Tree(action) => {
                            tree.map_action(action, gs);
                        }
                        Action::Editor(action) => {
                            if let Some(editor) = ws.get_active() {
                                editor.map(action, gs);
                            }
                        }
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

impl From<TreeAction> for Action {
    fn from(action: TreeAction) -> Self {
        Self::Tree(action)
    }
}

impl From<EditorAction> for Action {
    fn from(action: EditorAction) -> Self {
        Self::Editor(action)
    }
}
