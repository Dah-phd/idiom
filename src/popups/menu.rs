use super::{Components, Popup, Status};
use crate::configs::{EditorAction, TreeAction};
use crate::cursor::CursorPosition;
use crate::ext_tui::State;
use crate::global_state::GlobalState;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Rect, Backend, Position};

enum Action {
    Tree(TreeAction),
    Editor(EditorAction),
}

pub fn menu_context_editor_inplace(position: Position, screen: Rect, accent_style: ContentStyle) -> ContextMenu<7> {
    let modal_screen = screen.modal_relative(position.row, position.col, 30, 7);

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

pub fn menu_context_tree_inplace(position: CursorPosition, screen: Rect, accent_style: ContentStyle) -> ContextMenu<8> {
    let row_offset = position.line as u16;
    let col_offset = position.char as u16;
    let modal_screen = screen.modal_relative(row_offset, col_offset, 30, 8);

    ContextMenu {
        commands: [
            ("New", TreeAction::NewFile.into()),
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
    fn force_render(&mut self, gs: &mut GlobalState) {
        let backend = gs.backend();
        let reset_style = backend.get_style();
        backend.set_style(self.accent_style);
        self.state.render_list_padded(self.commands.iter().map(|c| c.0), self.modal_screen.iter_padded(1), backend);
        backend.set_style(reset_style);
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut super::Components) -> Status {
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
                return Status::Finished;
            }
            _ => return Status::Pending,
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut super::Components) -> Status {
        let Components { gs, ws, tree, .. } = components;
        match event.kind {
            MouseEventKind::Moved => {
                if let Some(position) = self.modal_screen.relative_position(event.row, event.column) {
                    let pos_line = position.row as usize;
                    if N > pos_line {
                        self.state.selected = pos_line;
                        self.force_render(gs);
                    };
                };
            }
            MouseEventKind::Down(MouseButton::Left | MouseButton::Right) => {
                if let Some(position) = self.modal_screen.relative_position(event.row, event.column) {
                    self.state.selected = position.row as usize;
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
                return Status::Finished;
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
