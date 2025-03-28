use super::{GlobalState, IdiomEvent, MIN_HEIGHT, MIN_WIDTH};
use crate::popups::menu::{menu_context_editor, menu_context_tree};
use crate::popups::pallet::Pallet;
use crate::render::backend::{Backend, StyleExt};
use crate::render::layout::Line;
use crate::{runner::EditorTerminal, tree::Tree, workspace::Workspace};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
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
            if let Some(position) = gs.tree_area.relative_position(event.row, event.column) {
                if let Some(path) = tree.mouse_select(position.line + 1, gs) {
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
            if let Some(position) = gs.tab_area.relative_position(event.row, event.column) {
                if !workspace.is_empty() {
                    gs.insert_mode();
                    if let Some(idx) = workspace.select_tab_mouse(position.char) {
                        workspace.activate_editor(idx, gs);
                        workspace.close_active(gs);
                    }
                }
            }
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = workspace.get_active() {
                    editor.mouse_menu_setup(position);
                    let accent_style = gs.theme.accent_style;
                    gs.popup(menu_context_editor(position, gs.editor_area, accent_style));
                }
            }
            if let Some(mut position) = gs.tree_area.relative_position(event.row, event.column) {
                position.line += 1;
                if tree.mouse_menu_setup_select(position.line) {
                    let accent_style = gs.theme.accent_style.reverse();
                    gs.popup(menu_context_tree(position, gs.screen_rect, accent_style));
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

pub fn map_small_rect(
    gs: &mut GlobalState,
    event: &KeyEvent,
    workspace: &mut Workspace,
    tree: &mut Tree,
    tmux: &mut EditorTerminal,
) -> bool {
    if gs.screen_rect.width < MIN_WIDTH || gs.screen_rect.height < MIN_HEIGHT {
        match event {
            KeyEvent { code: KeyCode::Char('q' | 'd' | 'Q' | 'D'), .. } => {
                gs.exit = true;
            }
            _ => (),
        }
        return true;
    }
    gs.config_controls();
    gs.map_key(event, workspace, tree, tmux)
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
    match gs.popup.paste_passthrough(clip, &gs.matcher) {
        PopupMessage::None => {}
        PopupMessage::Clear => {
            gs.clear_popup();
        }
        PopupMessage::Event(event) => {
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
