use super::{Components, InplacePopup, Popup, Status};
use crate::{
    configs::{EditorAction, TreeAction},
    cursor::CursorPosition,
    ext_tui::State,
    global_state::GlobalState,
    popups::{popup_find::FindPopup, popup_tree_search::ActivePathSearch},
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use idiom_tui::{Backend, Position, layout::Rect};

pub fn menu_context_editor_inplace(position: Position, screen: Rect, accent_style: ContentStyle) -> ContextMenu<9> {
    let commands = [
        ("Go to Definition", EditorAction::GoToDeclaration.into()),
        ("Find References", EditorAction::FindReferences.into()),
        ("Details / Info", EditorAction::Help.into()),
        ("Mark word", EditorAction::MarkWord.into()),
        ("Rename", EditorAction::LSPRename.into()),
        ("Find", Action::Popup(Box::new(FindPopup::run_inplace))),
        ("Cut", EditorAction::Cut.into()),
        ("Copy", EditorAction::Copy.into()),
        ("Paste", EditorAction::Paste.into()),
    ];
    let modal_screen = screen.modal_relative(position.row, position.col, 30, commands.len() as u16);

    ContextMenu { commands, modal_screen, accent_style, state: State::new() }
}

pub fn menu_context_tree_inplace(position: CursorPosition, screen: Rect, accent_style: ContentStyle) -> ContextMenu<9> {
    let row_offset = position.line as u16;
    let col_offset = position.char as u16;
    let commands = [
        ("New", TreeAction::NewFile.into()),
        ("Cut", TreeAction::CutFile.into()),
        ("Copy", TreeAction::CopyFile.into()),
        ("Paste", TreeAction::Paste.into()),
        ("Search", Action::Popup(Box::new(ActivePathSearch::run))),
        ("Copy Path", TreeAction::CopyPath.into()),
        ("Copy Relative Path", TreeAction::CopyPathRelative.into()),
        ("Rename", TreeAction::Rename.into()),
        ("Delete", TreeAction::Delete.into()),
    ];
    let modal_screen = screen.modal_relative(row_offset, col_offset, 30, commands.len() as u16);

    ContextMenu { commands, modal_screen, accent_style, state: State::with_highlight(ContentStyle::default()) }
}

enum Action {
    Tree(TreeAction),
    Editor(EditorAction),
    Popup(InplacePopup),
}

impl Action {
    fn execute(&mut self, components: &mut Components) {
        let Components { gs, ws, tree, term } = components;
        match self {
            Action::Tree(action) => {
                tree.map_action(*action, gs);
            }
            Action::Editor(action) => {
                if let Some(editor) = ws.get_active() {
                    editor.map(*action, gs);
                }
            }
            Action::Popup(cb) => cb(gs, ws, tree, term),
        }
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

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        match key {
            KeyEvent { code: KeyCode::Up, .. } => self.state.prev(N),
            KeyEvent { code: KeyCode::Down, .. } => self.state.next(N),
            KeyEvent { code: KeyCode::Enter, .. } => {
                self.commands[self.state.selected].1.execute(components);
                return Status::Finished;
            }
            _ => return Status::Pending,
        }
        self.force_render(components.gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status {
        match event.kind {
            MouseEventKind::Moved => {
                if let Some(position) = self.modal_screen.relative_position(event.row, event.column) {
                    let pos_line = position.row as usize;
                    if N > pos_line {
                        self.state.selected = pos_line;
                        self.force_render(components.gs);
                    };
                };
            }
            MouseEventKind::Down(MouseButton::Left | MouseButton::Right) => {
                if let Some(position) = self.modal_screen.relative_position(event.row, event.column) {
                    self.state.selected = position.row as usize;
                    self.commands[self.state.selected].1.execute(components);
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
