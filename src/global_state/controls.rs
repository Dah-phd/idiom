use super::{GlobalState, IdiomEvent};
use crate::ext_tui::{CrossTerm, StyleExt};
use crate::popups::menu::{menu_context_editor_inplace, menu_context_tree_inplace};
use crate::popups::pallet::Pallet;
use crate::popups::Popup;
use crate::{embeded_term::EditorTerminal, tree::Tree, workspace::Workspace};
use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::layout::Line;

const INSERT_SPAN: &str = "  --INSERT--  ";
const SELECT_SPAN: &str = "  --SELECT--  ";
const MODE_LEN: usize = INSERT_SPAN.len();

#[derive(Debug, Default, Clone, PartialEq)]
pub enum Mode {
    #[default]
    Select,
    Insert,
}

impl Mode {
    #[inline]
    pub fn render(&self, line: Line, backend: &mut CrossTerm) {
        match self {
            Self::Insert => Self::render_insert_mode(line, backend),
            Self::Select => Self::render_select_mode(line, backend),
        };
    }

    #[inline]
    pub fn render_select_mode(line: Line, backend: &mut CrossTerm) {
        let mut style = ContentStyle::reversed();
        style.add_bold();
        style.set_fg(Some(Self::select_color()));
        line.render_centered_styled(SELECT_SPAN, style, backend);
    }

    #[inline]
    pub fn render_insert_mode(line: Line, backend: &mut CrossTerm) {
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

pub fn mouse_term(
    event: MouseEvent,
    gs: &mut GlobalState,
    _ws: &mut Workspace,
    _tree: &mut Tree,
    term: &mut EditorTerminal,
) {
    term.map_mouse(event, gs);
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
                    editor.mouse_scroll_up(event.modifiers.contains(KeyModifiers::SHIFT), gs);
                }
            }
            Mode::Select => tree.select_up(gs),
        },
        MouseEventKind::ScrollDown => match gs.mode {
            Mode::Insert => {
                if let Some(editor) = ws.get_active() {
                    editor.mouse_scroll_down(event.modifiers.contains(KeyModifiers::SHIFT), gs);
                }
            }
            Mode::Select => tree.select_down(gs),
        },
        MouseEventKind::Moved => match gs.mode {
            Mode::Insert => {
                if let Some(editor) = ws.get_active() {
                    editor.mouse_moved(event.row, event.column, gs);
                }
            }
            Mode::Select => (), // no action
        },
        MouseEventKind::Down(MouseButton::Left) => {
            // on up currsor can drop select
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = ws.get_active() {
                    match event.modifiers {
                        KeyModifiers::ALT => editor.mouse_multi_cursor(position),
                        KeyModifiers::CONTROL => editor.mouse_select_to(position, gs),
                        _ => editor.mouse_click(position, gs),
                    };
                    gs.insert_mode();
                    match tree.select_by_path(&editor.path) {
                        Ok(..) => ws.toggle_editor(),
                        Err(error) => gs.error(error),
                    };
                }
            } else if let Some(position) = gs.tree_area.relative_position(event.row, event.column) {
                let pos_line = position.row as usize + 1;
                gs.event.push(IdiomEvent::SetMode(Mode::Select));
                if let Some(path) = tree.mouse_select(pos_line, gs) {
                    gs.event.push(IdiomEvent::OpenAtLine(path, 0));
                }
            } else if let Some(pos) = gs.tab_area.relative_position(event.row, event.column) {
                if !ws.is_empty() {
                    gs.select_mode();
                    let pos_char = pos.col as usize;
                    if let Some(idx) = ws.select_tab_mouse(pos_char) {
                        ws.activate_editor(idx, gs);
                    };
                }
            } else if gs.tree_area.relative_position(event.row + 2, event.column).is_some() {
                Pallet::run(gs, ws, tree, term);
            }
        }
        MouseEventKind::Up(MouseButton::Right) => {
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = ws.get_active() {
                    editor.clear_ui(gs);
                    editor.mouse_menu_setup(position);
                    let accent_style = gs.ui_theme.accent_style();
                    let mut context_menu = menu_context_editor_inplace(position, gs.editor_area, accent_style);
                    if let Err(error) = context_menu.run(gs, ws, tree, term) {
                        gs.error(error);
                    };
                };
            } else if let Some(position) = gs.tab_area.relative_position(event.row, event.column) {
                if !ws.is_empty() {
                    gs.insert_mode();
                    let pos_char = position.col as usize;
                    if let Some(idx) = ws.select_tab_mouse(pos_char) {
                        ws.activate_editor(idx, gs);
                        ws.close_active(gs);
                    }
                }
            } else if let Some(position) = gs.tree_area.relative_position(event.row, event.column) {
                let mut position = crate::workspace::CursorPosition::from(position);
                position.line += 1;
                if tree.mouse_menu_setup_select(position.line) {
                    let accent_style = gs.ui_theme.accent_style_reversed();
                    let mut context_menu = menu_context_tree_inplace(position, gs.screen_rect, accent_style);
                    if let Err(error) = context_menu.run(gs, ws, tree, term) {
                        gs.error(error);
                    };
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(position) = gs.editor_area.relative_position(event.row, event.column) {
                if let Some(editor) = ws.get_active() {
                    editor.mouse_select(position.into());
                    gs.insert_mode();
                    ws.toggle_editor();
                }
            }
        }
        _ => (),
    }
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
