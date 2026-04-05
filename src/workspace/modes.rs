use super::{TAB_SELECT, Workspace};
use crate::{configs::EditorAction, global_state::GlobalState};
use crossterm::event::KeyEvent;
use crossterm::style::{Attribute, Attributes, ContentStyle};

const TAB_MODE_STYLE: ContentStyle = ContentStyle {
    foreground_color: None,
    background_color: None,
    underline_color: None,
    attributes: Attributes::none().with(Attribute::Reverse),
};
const EDITOR_MODE_STYLE: ContentStyle = ContentStyle {
    foreground_color: Some(TAB_SELECT),
    background_color: None,
    underline_color: None,
    attributes: Attributes::none().with(Attribute::Underlined),
};

const TAB_MODE: Mode = Mode { key_map: map_tabs, style: TAB_MODE_STYLE };
const EDITOR_MODE: Mode = Mode { key_map: map_editor_post_save, style: EDITOR_MODE_STYLE };
const EDITOR_UPDATED_MODE: Mode = Mode { key_map: map_editor, style: EDITOR_MODE_STYLE };

pub struct Mode {
    key_map: fn(&mut Workspace, &KeyEvent, &mut GlobalState) -> bool,
    style: ContentStyle,
}

impl Mode {
    pub const fn new_editor() -> Self {
        EDITOR_MODE
    }

    #[allow(dead_code)]
    pub const fn new_tab() -> Self {
        TAB_MODE
    }

    #[inline(always)]
    pub fn map(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        (ws.mode.key_map)(ws, key, gs)
    }

    #[inline(always)]
    pub fn style(&self) -> ContentStyle {
        self.style
    }

    #[inline]
    pub fn is_editor(&self) -> bool {
        !self.is_tab()
    }

    #[inline]
    pub fn is_tab(&self) -> bool {
        self.style == TAB_MODE_STYLE
    }
}

/// Handles keybinding while on tabs
fn map_tabs(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
    if let Some(action) = ws.key_map.map(key) {
        if ws.editors.is_empty() {
            gs.select_mode();
            return false;
        }
        match action {
            EditorAction::NewLine => {
                ws.toggle_editor();
                if let Some(editor) = ws.get_active() {
                    editor.clear_ui(gs);
                }
            }
            EditorAction::Up | EditorAction::Down => {
                ws.toggle_editor();
                gs.select_mode();
                return false;
            }
            EditorAction::Right | EditorAction::Indent if ws.editors.len() > 1 => {
                let editor = ws.editors.remove(0);
                ws.editors.push(editor);
                let editor = &mut ws.editors.inner_mut_no_update()[0];
                editor.clear_screen_cache(gs);
                gs.select_editor_events(editor);
            }
            EditorAction::Left | EditorAction::Unintent if ws.editors.len() > 1 => {
                let mut editor = ws.editors.remove(ws.editors.len() - 1);
                editor.clear_screen_cache(gs);
                gs.select_editor_events(&editor);
                ws.editors.insert(0, editor);
            }
            EditorAction::Cancel => {
                ws.toggle_editor();
                if let Some(editor) = ws.get_active() {
                    editor.clear_ui(gs);
                }
                return false;
            }
            EditorAction::Close => {
                ws.close_active(gs);
            }
            _ => (),
        }
        return true;
    }
    false
}

/// handels keybindings for editor post save
/// if editor is updated it will switch to UPDAETD
/// and will no logner check for updates to trigger render
fn map_editor_post_save(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
    let result = map_editor(ws, key, gs);
    // mode has been switched
    if ws.mode.is_tab() {
        return result;
    }
    // no editor to map
    let Some(editor) = ws.get_active() else { return result };
    // no updates on editor
    if editor.is_saved() {
        return result;
    }
    // prevent further checks and force render
    ws.mode = EDITOR_UPDATED_MODE;
    ws.editors.mark_updated();
    result
}

/// handels keybindings for editor
fn map_editor(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
    let Some(editor) = ws.editors.get_mut_no_update(0) else {
        return false;
    };
    let Some(action) = ws.key_map.map(key) else {
        return false;
    };
    if !editor.map(action, gs) {
        match action {
            EditorAction::Save => {
                if !ws.base_configs.format_on_save || !editor.try_formatter_and_save(gs) {
                    editor.save(gs);
                }
                ws.editors.mark_updated();
            }
            EditorAction::Close => ws.close_active(gs),
            EditorAction::Cancel if ws.editors.len() > 1 => ws.toggle_tabs(),
            _ => return false,
        }
    }
    true
}
