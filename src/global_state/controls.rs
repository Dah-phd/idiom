use crate::{
    global_state::{GlobalState, Mode, TreeEvent},
    runner::EditorTerminal,
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::KeyEvent;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::prelude::Rect;

type Line = usize;
type Column = usize;

pub fn contained_position(rect: Rect, row: u16, column: u16) -> Option<(Line, Column)> {
    if rect.x <= column && column <= rect.width && rect.y <= row && row <= rect.height {
        return Some(((row - rect.y) as usize, (column - rect.x) as usize));
    }
    None
}

#[allow(clippy::needless_return)]
pub fn mouse_handler(gs: &mut GlobalState, event: MouseEvent, tree: &mut Tree, workspace: &mut Workspace) {
    match event.kind {
        MouseEventKind::ScrollUp if matches!(gs.mode, Mode::Insert) => {
            if let Some(editor) = workspace.get_active() {
                editor.scroll_up();
                editor.scroll_up();
            }
        }
        MouseEventKind::ScrollDown if matches!(gs.mode, Mode::Insert) => {
            if let Some(editor) = workspace.get_active() {
                editor.scroll_down();
                editor.scroll_down();
            }
        }
        MouseEventKind::Up(_button) => {
            //TODO figure out how to use
        }
        MouseEventKind::Down(button) => {
            if matches!(button, MouseButton::Right) {
                if let Some((_, col_idx)) = contained_position(gs.tab_area, event.row, event.column) {
                    if !workspace.editors.is_empty() {
                        gs.insert_mode();
                        if let Some(idx) = workspace.select_tab_mouse(col_idx) {
                            workspace.activate_editor(idx, None);
                            workspace.close_active();
                            if workspace.editors.is_empty() {
                                gs.select_mode();
                            }
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
            if !matches!(button, MouseButton::Left) {
                return;
            }
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
                if let Some(path) = tree.mouse_select(line_idx) {
                    gs.tree.push(TreeEvent::Open(path));
                    return;
                };
                gs.select_mode();
            }
            if let Some((_, col_idx)) = contained_position(gs.tab_area, event.row, event.column) {
                if !workspace.editors.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = workspace.select_tab_mouse(col_idx) {
                        workspace.activate_editor(idx, Some(gs));
                    };
                }
            }
        }
        MouseEventKind::Drag(button) => {
            if !matches!(button, MouseButton::Left) {
                return;
            }
            if let Some(position) = contained_position(gs.editor_area, event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    editor.mouse_select(position.into());
                    gs.insert_mode();
                    workspace.toggle_editor();
                }
                return;
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
