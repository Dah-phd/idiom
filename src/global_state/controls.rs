use super::{GlobalState, IdiomEvent, Mode};
use crate::{render::layout::Rect, runner::EditorTerminal, tree::Tree, workspace::Workspace};
use crossterm::event::KeyEvent;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

type Row = usize;
type Col = usize;

pub fn contained_position(rect: Rect, row: u16, column: u16) -> Option<(Row, Col)> {
    if rect.col <= column && column <= rect.width as u16 && rect.row <= row && row <= rect.height {
        return Some(((row - rect.row) as usize, (column - rect.col) as usize));
    }
    None
}

#[allow(clippy::needless_return)]
pub fn mouse_handler(gs: &mut GlobalState, event: MouseEvent, tree: &mut Tree, workspace: &mut Workspace) {
    match event.kind {
        MouseEventKind::ScrollUp if matches!(gs.mode, Mode::Insert) => {
            if let Some(editor) = workspace.get_active() {
                editor.map(crate::configs::EditorAction::ScrollUp, gs);
                editor.map(crate::configs::EditorAction::ScrollUp, gs);
            }
        }
        MouseEventKind::ScrollDown if matches!(gs.mode, Mode::Insert) => {
            if let Some(editor) = workspace.get_active() {
                editor.map(crate::configs::EditorAction::ScrollDown, gs);
                editor.map(crate::configs::EditorAction::ScrollDown, gs);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(position) = contained_position(gs.editor_area, event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    editor.mouse_cursor(position.into());
                    gs.insert_mode();
                    tree.select_by_path(&editor.path);
                    workspace.toggle_editor();
                }
                return;
            }
            if let Some((line_idx, _)) = contained_position(gs.tree_area, event.row, event.column) {
                if let Some(path) = tree.mouse_select(line_idx + 1) {
                    gs.event.push(IdiomEvent::Open(path));
                    return;
                };
                gs.select_mode();
            }
            if let Some((_, col_idx)) = contained_position(gs.tab_area, event.row, event.column) {
                if !workspace.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = workspace.select_tab_mouse(col_idx) {
                        workspace.activate_editor(idx, gs);
                    };
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some((_, col_idx)) = contained_position(gs.tab_area, event.row, event.column) {
                if !workspace.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = workspace.select_tab_mouse(col_idx) {
                        workspace.activate_editor(idx, gs);
                        workspace.close_active(gs);
                    }
                }
            }
            if let Some(position) = contained_position(gs.editor_area, event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    if let Some(clip) = editor.mouse_copy_paste(position.into(), gs.clipboard.pull()) {
                        gs.clipboard.push(clip);
                        gs.success("Copied select!");
                    };
                    gs.insert_mode();
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(position) = contained_position(gs.editor_area, event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    editor.mouse_select(position.into());
                    gs.insert_mode();
                    workspace.toggle_editor();
                }
            }
        }
        _ => (),
    }
}

pub fn disable_mouse(_gs: &mut GlobalState, _event: MouseEvent, _tree: &mut Tree, _workspace: &mut Workspace) {}

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
