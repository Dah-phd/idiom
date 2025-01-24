use super::{GlobalState, IdiomEvent};
use crate::popups::pallet::Pallet;
use crate::render::backend::{Backend, StyleExt};
use crate::render::layout::Line;
use crate::{runner::EditorTerminal, tree::Tree, workspace::Workspace};
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::{Color, ContentStyle};

const INSERT_SPAN: &str = "  --INSERT--   ";
const SELECT_SPAN: &str = "  --SELECT--   ";
const MODE_LEN: usize = INSERT_SPAN.len();

#[derive(Default, Clone)]
pub enum PopupMessage {
    #[default]
    None,
    Event(IdiomEvent),
    Clear,
}

#[derive(Default)]
pub enum Mode {
    #[default]
    Select,
    Insert,
}

impl Mode {
    #[inline]
    pub fn render(&self, line: Line, accent_style: ContentStyle, backend: &mut Backend) {
        match self {
            Self::Insert => Self::render_insert_mode(line, accent_style, backend),
            Self::Select => Self::render_select_mode(line, accent_style, backend),
        };
    }

    #[inline]
    pub fn render_select_mode(mut line: Line, mut accent_style: ContentStyle, backend: &mut Backend) {
        line.width = std::cmp::min(MODE_LEN, line.width);
        accent_style.add_bold();
        accent_style.set_fg(Some(Self::select_color()));
        line.render_styled(SELECT_SPAN, accent_style, backend);
    }

    #[inline]
    pub fn render_insert_mode(mut line: Line, mut accent_style: ContentStyle, backend: &mut Backend) {
        line.width = std::cmp::min(MODE_LEN, line.width);
        accent_style.add_bold();
        accent_style.set_fg(Some(Self::insert_color()));
        line.render_styled(INSERT_SPAN, accent_style, backend);
    }

    pub const fn select_color() -> Color {
        Color::Cyan
    }

    pub const fn insert_color() -> Color {
        Color::Rgb { r: 255, g: 0, b: 0 }
    }

    #[inline]
    pub const fn len() -> usize {
        MODE_LEN
    }
}

pub fn disable_mouse(_gs: &mut GlobalState, _event: MouseEvent, _tree: &mut Tree, _workspace: &mut Workspace) {}

pub fn mouse_handler(gs: &mut GlobalState, event: MouseEvent, tree: &mut Tree, workspace: &mut Workspace) {
    match event.kind {
        MouseEventKind::ScrollUp => match gs.mode {
            Mode::Insert => {
                if let Some(editor) = workspace.get_active() {
                    editor.map(crate::configs::EditorAction::ScrollUp, gs);
                    editor.map(crate::configs::EditorAction::ScrollUp, gs);
                }
            }
            Mode::Select => tree.select_up(gs),
        },
        MouseEventKind::ScrollDown => match gs.mode {
            Mode::Insert => {
                if let Some(editor) = workspace.get_active() {
                    editor.map(crate::configs::EditorAction::ScrollDown, gs);
                    editor.map(crate::configs::EditorAction::ScrollDown, gs);
                }
            }
            Mode::Select => tree.select_down(gs),
        },
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    editor.mouse_cursor(position);
                    gs.insert_mode();
                    match tree.select_by_path(&editor.path) {
                        Ok(..) => workspace.toggle_editor(),
                        Err(error) => gs.error(error),
                    };
                }
                return;
            }
            if let Some(pos) = gs.tree_area.relative_position(event.row, event.column) {
                if let Some(path) = tree.mouse_select(pos.line + 1, gs) {
                    gs.event.push(IdiomEvent::OpenAtLine(path, 0));
                    return;
                };
                gs.select_mode();
                return;
            };
            if let Some(pos) = gs.tab_area.relative_position(event.row, event.column) {
                if !workspace.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = workspace.select_tab_mouse(pos.char) {
                        workspace.activate_editor(idx, gs);
                    };
                }
                return;
            }
            if gs.tree_area.relative_position(event.row + 2, event.column).is_some() {
                gs.popup(Pallet::new());
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some(pos) = gs.tab_area.relative_position(event.row, event.column) {
                if !workspace.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = workspace.select_tab_mouse(pos.char) {
                        workspace.activate_editor(idx, gs);
                        workspace.close_active(gs);
                    }
                }
            }
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    if let Some(clip) = editor.mouse_copy_paste(position, gs.clipboard.pull()) {
                        gs.clipboard.push(clip);
                        gs.success("Copied select!");
                    };
                    gs.insert_mode();
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    editor.mouse_select(position);
                    gs.insert_mode();
                    workspace.toggle_editor();
                }
            }
        }
        _ => (),
    }
}

pub fn mouse_popup_handler(gs: &mut GlobalState, event: MouseEvent, _tree: &mut Tree, _workspace: &mut Workspace) {
    match gs.popup.mouse_map(event) {
        PopupMessage::None => {}
        PopupMessage::Clear => {
            gs.clear_popup();
        }
        PopupMessage::Event(event) => {
            gs.event.push(event);
        }
    };
}

pub fn map_editor(
    gs: &mut GlobalState,
    key: &KeyEvent,
    workspace: &mut Workspace,
    _t: &mut Tree,
    _r: &mut EditorTerminal,
) -> bool {
    workspace.map(key, gs)
}

pub fn map_tree(
    gs: &mut GlobalState,
    key: &KeyEvent,
    _w: &mut Workspace,
    tree: &mut Tree,
    _r: &mut EditorTerminal,
) -> bool {
    tree.map(key, gs)
}

pub fn map_popup(
    gs: &mut GlobalState,
    key: &KeyEvent,
    _w: &mut Workspace,
    _t: &mut Tree,
    _r: &mut EditorTerminal,
) -> bool {
    gs.map_popup_if_exists(key)
}

pub fn map_term(
    gs: &mut GlobalState,
    key: &KeyEvent,
    _w: &mut Workspace,
    _t: &mut Tree,
    runner: &mut EditorTerminal,
) -> bool {
    runner.map(key, gs)
}

#[cfg(test)]
mod test {
    use super::{INSERT_SPAN, MODE_LEN, SELECT_SPAN};

    #[test]
    fn ensure_mode_len_match() {
        assert_eq!(INSERT_SPAN.len(), SELECT_SPAN.len());
        assert_eq!(INSERT_SPAN.len(), MODE_LEN);
    }
}
