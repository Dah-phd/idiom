use super::{GlobalState, IdiomEvent};
use crate::popups::menu::{menu_context_editor_inplace, menu_context_tree_inplace};
use crate::popups::pallet::Pallet;
use crate::popups::InplacePopup;
use crate::render::backend::{Backend, StyleExt};
use crate::render::layout::Line;
use crate::{embeded_term::EditorTerminal, tree::Tree, workspace::Workspace};
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::{Color, ContentStyle, Stylize};

const INSERT_SPAN: &str = "  --INSERT--  ";
const SELECT_SPAN: &str = "  --SELECT--  ";
const MODE_LEN: usize = INSERT_SPAN.len();

#[derive(Default, Debug, Clone)]
pub enum PopupMessage {
    #[default]
    None,
    Event(IdiomEvent),
    Clear,
    ClearEvent(IdiomEvent),
}

#[derive(Default)]
pub enum Mode {
    #[default]
    Select,
    Insert,
}

impl Mode {
    #[inline]
    pub fn render(&self, line: Line, backend: &mut Backend) {
        match self {
            Self::Insert => Self::render_insert_mode(line, backend),
            Self::Select => Self::render_select_mode(line, backend),
        };
    }

    #[inline]
    pub fn render_select_mode(line: Line, backend: &mut Backend) {
        let mut style = ContentStyle::reversed();
        style.add_bold();
        style.set_fg(Some(Self::select_color()));
        line.render_centered_styled(SELECT_SPAN, style, backend);
    }

    #[inline]
    pub fn render_insert_mode(line: Line, backend: &mut Backend) {
        let mut style = ContentStyle::reversed();
        style.add_bold();
        style.set_fg(Some(Self::insert_color()));
        line.render_centered_styled(INSERT_SPAN, style, backend);
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

pub fn disable_mouse(
    _event: MouseEvent,
    _gs: &mut GlobalState,
    _ws: &mut Workspace,
    _tree: &mut Tree,
    _term: &mut EditorTerminal,
) {
}

pub fn mouse_handler(
    event: MouseEvent,
    gs: &mut GlobalState,
    ws: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) {
    match event.kind {
        MouseEventKind::ScrollUp => match gs.mode {
            Mode::Insert => {
                if let Some(editor) = ws.get_active() {
                    editor.map(crate::configs::EditorAction::ScrollUp, gs);
                    editor.map(crate::configs::EditorAction::ScrollUp, gs);
                }
            }
            Mode::Select => tree.select_up(gs),
        },
        MouseEventKind::ScrollDown => match gs.mode {
            Mode::Insert => {
                if let Some(editor) = ws.get_active() {
                    editor.map(crate::configs::EditorAction::ScrollDown, gs);
                    editor.map(crate::configs::EditorAction::ScrollDown, gs);
                }
            }
            Mode::Select => tree.select_down(gs),
        },
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = ws.get_active() {
                    editor.mouse_cursor(position);
                    gs.insert_mode();
                    match tree.select_by_path(&editor.path) {
                        Ok(..) => ws.toggle_editor(),
                        Err(error) => gs.error(error),
                    };
                }
                return;
            }
            if let Some(position) = gs.tree_area.relative_position(event.row, event.column) {
                if let Some(path) = tree.mouse_select(position.line + 1, gs) {
                    gs.event.push(IdiomEvent::OpenAtLine(path, 0));
                    return;
                };
                gs.select_mode();
                return;
            };
            if let Some(pos) = gs.tab_area.relative_position(event.row, event.column) {
                if !ws.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = ws.select_tab_mouse(pos.char) {
                        ws.activate_editor(idx, gs);
                    };
                }
                return;
            }
            if gs.tree_area.relative_position(event.row + 2, event.column).is_some() {
                Pallet::new(gs.screen_rect).run(gs, ws, tree, term);
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some(position) = gs.tab_area.relative_position(event.row, event.column) {
                if !ws.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = ws.select_tab_mouse(position.char) {
                        ws.activate_editor(idx, gs);
                        ws.close_active(gs);
                    }
                }
            }
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = ws.get_active() {
                    editor.mouse_menu_setup(position);
                    let accent_style = gs.theme.accent_style;
                    menu_context_editor_inplace(position, gs.editor_area, accent_style).run(gs, ws, tree, term);
                }
            }
            if let Some(mut position) = gs.tree_area.relative_position(event.row, event.column) {
                position.line += 1;
                if tree.mouse_menu_setup_select(position.line) {
                    let accent_style = gs.theme.accent_style.reverse();
                    menu_context_tree_inplace(position, gs.screen_rect, accent_style).run(gs, ws, tree, term);
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = ws.get_active() {
                    editor.mouse_select(position);
                    gs.insert_mode();
                    ws.toggle_editor();
                }
            }
        }
        _ => (),
    }
}

pub fn mouse_popup_handler(
    event: MouseEvent,
    gs: &mut GlobalState,
    _workspace: &mut Workspace,
    _tree: &mut Tree,
    _term: &mut EditorTerminal,
) {
    let Some(popup) = gs.popup.as_mut() else {
        gs.config_controls();
        return;
    };
    match popup.mouse_map(event) {
        PopupMessage::None => {}
        PopupMessage::Clear => {
            gs.clear_popup();
        }
        PopupMessage::Event(event) => {
            gs.event.push(event);
        }
        PopupMessage::ClearEvent(event) => {
            gs.clear_popup();
            gs.event.push(event);
        }
    };
}

pub fn map_editor(
    key: &KeyEvent,
    gs: &mut GlobalState,
    workspace: &mut Workspace,
    _t: &mut Tree,
    _r: &mut EditorTerminal,
) -> bool {
    workspace.map(key, gs)
}

pub fn map_tree(
    key: &KeyEvent,
    gs: &mut GlobalState,
    _w: &mut Workspace,
    tree: &mut Tree,
    _r: &mut EditorTerminal,
) -> bool {
    tree.map(key, gs)
}

pub fn map_popup(
    key: &KeyEvent,
    gs: &mut GlobalState,
    _w: &mut Workspace,
    _t: &mut Tree,
    _r: &mut EditorTerminal,
) -> bool {
    gs.map_popup_if_exists(key)
}

pub fn map_term(
    key: &KeyEvent,
    gs: &mut GlobalState,
    _w: &mut Workspace,
    _t: &mut Tree,
    runner: &mut EditorTerminal,
) -> bool {
    runner.map(key, gs)
}

pub fn paste_passthrough_editor(
    _gs: &mut GlobalState,
    clip: String,
    workspace: &mut Workspace,
    _tmux: &mut EditorTerminal,
) {
    if let Some(editor) = workspace.get_active() {
        editor.paste(clip);
    }
}

pub fn paste_passthrough_popup(gs: &mut GlobalState, clip: String, _ws: &mut Workspace, _t: &mut EditorTerminal) {
    let Some(popup) = gs.popup.as_mut() else {
        gs.config_controls();
        return;
    };
    match popup.paste_passthrough(clip, &gs.matcher) {
        PopupMessage::None => {}
        PopupMessage::Clear => {
            gs.clear_popup();
        }
        PopupMessage::Event(event) => {
            gs.event.push(event);
        }
        PopupMessage::ClearEvent(event) => {
            gs.clear_popup();
            gs.event.push(event);
        }
    };
}

pub fn paste_passthrough_term(_gs: &mut GlobalState, clip: String, _ws: &mut Workspace, term: &mut EditorTerminal) {
    term.paste_passthrough(clip);
}

pub fn paste_passthrough_ignore(_gs: &mut GlobalState, _clip: String, _ws: &mut Workspace, _term: &mut EditorTerminal) {
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
