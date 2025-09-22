use super::defaults::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// EDITOR
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorAction {
    Char(char),
    NewLine,
    Indent,
    Backspace,
    Delete,
    RemoveLine,
    IndentStart,
    Unintent,
    Up,
    Down,
    Left,
    Right,
    SelectUp,
    SelectDown,
    SelectLeft,
    SelectRight,
    SelectToken,
    SelectLine,
    SelectAll,
    ScrollUp,
    ScrollDown,
    SelectScrollUp,
    SelectScrollDown,
    ScreenUp,
    ScreenDown,
    NewCursorUp,
    NewCursorDown,
    NewCursorWithLine,
    SwapUp,
    SwapDown,
    JumpLeft,
    JumpLeftSelect,
    JumpRight,
    JumpRightSelect,
    EndOfLine,
    EndOfFile,
    StartOfLine,
    StartOfFile,
    FindReferences,
    GoToDeclaration,
    Help,
    LSPRename,
    RefreshUI,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    Save,
    Cancel,
    Close,
    CommentOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorUserKeyMap {
    new_line_or_select: String,
    indent: String,
    backspace: String,
    delete: String,
    remove_line: String,
    indent_start: String,
    unindent: String,
    up: String,
    down: String,
    left: String,
    right: String,
    select_up: String,
    select_down: String,
    select_left: String,
    select_right: String,
    select_token: String,
    select_line: String,
    select_all: String,
    scroll_up: String,
    select_scroll_up: String,
    scroll_down: String,
    select_scroll_down: String,
    screen_up: String,
    screen_down: String,
    new_cursor_up: String,
    new_cursor_down: String,
    new_cursor_with_line: String,
    swap_up: String,
    swap_down: String,
    jump_left: String,
    jump_left_select: String,
    jump_right: String,
    jump_right_select: String,
    end_of_line: String,
    end_of_file: String,
    start_of_line: String,
    start_of_file: String,
    find_references: String,
    go_to_declaration: String,
    help: String,
    refresh_ui: String,
    lsp_rename: String,
    cut: String,
    copy: String,
    paste: String,
    undo: String,
    redo: String,
    save: String,
    cancel: String,
    close: String,
    comment_out: String,
}

impl From<EditorUserKeyMap> for HashMap<KeyEvent, EditorAction> {
    fn from(val: EditorUserKeyMap) -> Self {
        let mut hash = HashMap::default();
        insert_key_event(&mut hash, &val.new_line_or_select, EditorAction::NewLine);
        insert_key_event(&mut hash, &val.indent, EditorAction::Indent);
        insert_key_event(&mut hash, &val.backspace, EditorAction::Backspace);
        insert_key_event(&mut hash, &val.delete, EditorAction::Delete);
        insert_key_event(&mut hash, &val.remove_line, EditorAction::RemoveLine);
        insert_key_event(&mut hash, &val.indent_start, EditorAction::IndentStart);
        insert_key_event(&mut hash, &val.unindent, EditorAction::Unintent);
        insert_key_event(&mut hash, &val.up, EditorAction::Up);
        insert_key_event(&mut hash, &val.down, EditorAction::Down);
        insert_key_event(&mut hash, &val.left, EditorAction::Left);
        insert_key_event(&mut hash, &val.right, EditorAction::Right);
        insert_key_event(&mut hash, &val.select_up, EditorAction::SelectUp);
        insert_key_event(&mut hash, &val.select_down, EditorAction::SelectDown);
        insert_key_event(&mut hash, &val.select_left, EditorAction::SelectLeft);
        insert_key_event(&mut hash, &val.select_right, EditorAction::SelectRight);
        insert_key_event(&mut hash, &val.select_token, EditorAction::SelectToken);
        insert_key_event(&mut hash, &val.select_line, EditorAction::SelectLine);
        insert_key_event(&mut hash, &val.select_all, EditorAction::SelectAll);
        insert_key_event(&mut hash, &val.scroll_up, EditorAction::ScrollUp);
        insert_key_event(&mut hash, &val.select_scroll_up, EditorAction::SelectScrollUp);
        insert_key_event(&mut hash, &val.scroll_down, EditorAction::ScrollDown);
        insert_key_event(&mut hash, &val.select_scroll_down, EditorAction::SelectScrollDown);
        insert_key_event(&mut hash, &val.new_cursor_up, EditorAction::NewCursorUp);
        insert_key_event(&mut hash, &val.new_cursor_down, EditorAction::NewCursorDown);
        insert_key_event(&mut hash, &val.new_cursor_with_line, EditorAction::NewCursorWithLine);
        insert_key_event(&mut hash, &val.swap_up, EditorAction::SwapUp);
        insert_key_event(&mut hash, &val.swap_down, EditorAction::SwapDown);
        insert_key_event(&mut hash, &val.screen_up, EditorAction::ScreenUp);
        insert_key_event(&mut hash, &val.screen_down, EditorAction::ScreenDown);
        insert_key_event(&mut hash, &val.jump_left, EditorAction::JumpLeft);
        insert_key_event(&mut hash, &val.jump_left_select, EditorAction::JumpLeftSelect);
        insert_key_event(&mut hash, &val.jump_right, EditorAction::JumpRight);
        insert_key_event(&mut hash, &val.jump_right_select, EditorAction::JumpRightSelect);
        insert_key_event(&mut hash, &val.end_of_line, EditorAction::EndOfLine);
        insert_key_event(&mut hash, &val.end_of_file, EditorAction::EndOfFile);
        insert_key_event(&mut hash, &val.start_of_line, EditorAction::StartOfLine);
        insert_key_event(&mut hash, &val.start_of_file, EditorAction::StartOfFile);
        insert_key_event(&mut hash, &val.find_references, EditorAction::FindReferences);
        insert_key_event(&mut hash, &val.go_to_declaration, EditorAction::GoToDeclaration);
        insert_key_event(&mut hash, &val.help, EditorAction::Help);
        insert_key_event(&mut hash, &val.lsp_rename, EditorAction::LSPRename);
        insert_key_event(&mut hash, &val.cut, EditorAction::Cut);
        insert_key_event(&mut hash, &val.copy, EditorAction::Copy);
        insert_key_event(&mut hash, &val.paste, EditorAction::Paste);
        insert_key_event(&mut hash, &val.undo, EditorAction::Undo);
        insert_key_event(&mut hash, &val.redo, EditorAction::Redo);
        insert_key_event(&mut hash, &val.save, EditorAction::Save);
        insert_key_event(&mut hash, &val.cancel, EditorAction::Cancel);
        insert_key_event(&mut hash, &val.close, EditorAction::Close);
        insert_key_event(&mut hash, &val.comment_out, EditorAction::CommentOut);
        hash
    }
}

impl Default for EditorUserKeyMap {
    fn default() -> Self {
        Self {
            new_line_or_select: ENTER.to_owned(),
            indent: TAB.to_owned(),
            backspace: BACKSPACE.to_owned(),
            delete: DELETE.to_owned(),
            remove_line: format!("{SHIFT} && {DELETE}"),
            indent_start: format!("{CTRL} && ]"),
            unindent: format!("{SHIFT} && {TAB}"),
            up: UP.to_owned(),
            down: DOWN.to_owned(),
            left: LEFT.to_owned(),
            right: RIGHT.to_owned(),
            select_up: format!("{SHIFT} && {UP}"),
            select_down: format!("{SHIFT} && {DOWN}"),
            select_left: format!("{SHIFT} && {LEFT}"),
            select_right: format!("{SHIFT} && {RIGHT}"),
            select_token: format!("{CTRL} && w"),
            select_line: format!("{CTRL} && l"),
            select_all: format!("{CTRL} && a"),
            scroll_up: format!("{CTRL} && {UP}"),
            scroll_down: format!("{CTRL} && {DOWN}"),
            select_scroll_up: format!("{CTRL} && {SHIFT} && {UP}"),
            select_scroll_down: format!("{CTRL} && {SHIFT} && {DOWN}"),
            screen_up: PAGEUP.to_owned(),
            screen_down: PAGEDOWN.to_owned(),
            new_cursor_up: format!("{ALT} && {CTRL} && {UP}"),
            new_cursor_down: format!("{ALT} && {CTRL} && {DOWN}"),
            new_cursor_with_line: format!("{ALT} && {ENTER}"),
            swap_up: format!("{ALT} && {UP}"),
            swap_down: format!("{ALT} && {DOWN}"),
            jump_left: format!("{CTRL} && {LEFT} || {ALT} && {LEFT}"),
            jump_left_select: format!("{CTRL} && {SHIFT} && {LEFT} || {ALT} && {SHIFT} && {LEFT}"),
            jump_right: format!("{CTRL} && {RIGHT} || {ALT} && {RIGHT}"),
            jump_right_select: format!("{CTRL} && {SHIFT} && {RIGHT} || {ALT} && {SHIFT} && {RIGHT}"),
            end_of_line: END.to_owned(),
            end_of_file: format!("{CTRL} && {END}"),
            start_of_line: HOME.to_owned(),
            start_of_file: format!("{CTRL} && {HOME}"),
            find_references: format!("{F}9"),
            go_to_declaration: format!("{F}12"),
            help: format!("{F}1"),
            refresh_ui: format!("{F}5"),
            lsp_rename: format!("{F}2"),
            cut: format!("{CTRL} && x"),
            copy: format!("{CTRL} && c || {CTRL} && {SHIFT} && c"),
            paste: format!("{CTRL} && v"),
            undo: format!("{CTRL} && z"),
            redo: format!("{CTRL} && y"),
            save: format!("{CTRL} && s"),
            cancel: ESC.to_owned(),
            close: format!("{CTRL} && d"),
            comment_out: format!("{CTRL} && /"),
        }
    }
}

// TREE

#[derive(Debug, Clone, Copy)]
pub enum GeneralAction {
    GoToTabs,
    InvokePallet,
    SelectOpenEditor,
    SaveAll,
    FileTreeModeOrCancelInput,
    Find,
    Replace,
    Exit,
    HideFileTree,
    RefreshSettings,
    GoToLine,
    GitTui,
    ToggleTerminal,
    ContextMenu,
    GoToTab1,
    GoToTab2,
    GoToTab3,
    GoToTab4,
    GoToTab5,
    GoToTab6,
    GoToTab7,
    GoToTab8,
    GoToTab9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralUserKeyMap {
    go_to_editor_tabs: String,
    invoke_pallet: String,
    select_open_editor: String,
    save_all: String,
    cancel: String,
    find: String,
    replace: String,
    backspace_tree_input: String,
    exit: String,
    hide_file_tree: String,
    refresh_settings: String,
    go_to_line: String,
    git_tui: String,
    toggle_terminal: String,
    context_menu: String,
    go_to_tab_1: String,
    go_to_tab_2: String,
    go_to_tab_3: String,
    go_to_tab_4: String,
    go_to_tab_5: String,
    go_to_tab_6: String,
    go_to_tab_7: String,
    go_to_tab_8: String,
    go_to_tab_9: String,
}

impl From<GeneralUserKeyMap> for HashMap<KeyEvent, GeneralAction> {
    fn from(val: GeneralUserKeyMap) -> Self {
        let mut hash = HashMap::default();
        insert_key_event(&mut hash, &val.go_to_editor_tabs, GeneralAction::GoToTabs);
        insert_key_event(&mut hash, &val.invoke_pallet, GeneralAction::InvokePallet);
        insert_key_event(&mut hash, &val.select_open_editor, GeneralAction::SelectOpenEditor);
        insert_key_event(&mut hash, &val.save_all, GeneralAction::SaveAll);
        insert_key_event(&mut hash, &val.cancel, GeneralAction::FileTreeModeOrCancelInput);
        insert_key_event(&mut hash, &val.find, GeneralAction::Find);
        insert_key_event(&mut hash, &val.replace, GeneralAction::Replace);
        insert_key_event(&mut hash, &val.exit, GeneralAction::Exit);
        insert_key_event(&mut hash, &val.hide_file_tree, GeneralAction::HideFileTree);
        insert_key_event(&mut hash, &val.refresh_settings, GeneralAction::RefreshSettings);
        insert_key_event(&mut hash, &val.go_to_line, GeneralAction::GoToLine);
        insert_key_event(&mut hash, &val.git_tui, GeneralAction::GitTui);
        insert_key_event(&mut hash, &val.toggle_terminal, GeneralAction::ToggleTerminal);
        insert_key_event(&mut hash, &val.context_menu, GeneralAction::ContextMenu);
        insert_key_event(&mut hash, &val.go_to_tab_1, GeneralAction::GoToTab1);
        insert_key_event(&mut hash, &val.go_to_tab_2, GeneralAction::GoToTab2);
        insert_key_event(&mut hash, &val.go_to_tab_3, GeneralAction::GoToTab3);
        insert_key_event(&mut hash, &val.go_to_tab_4, GeneralAction::GoToTab4);
        insert_key_event(&mut hash, &val.go_to_tab_5, GeneralAction::GoToTab5);
        insert_key_event(&mut hash, &val.go_to_tab_6, GeneralAction::GoToTab6);
        insert_key_event(&mut hash, &val.go_to_tab_7, GeneralAction::GoToTab7);
        insert_key_event(&mut hash, &val.go_to_tab_8, GeneralAction::GoToTab8);
        insert_key_event(&mut hash, &val.go_to_tab_9, GeneralAction::GoToTab9);
        hash
    }
}

impl Default for GeneralUserKeyMap {
    fn default() -> Self {
        Self {
            go_to_editor_tabs: TAB.to_owned(),
            invoke_pallet: format!("{CTRL} && p || {CTRL} && {SHIFT} && p"),
            select_open_editor: format!("{CTRL} && {TAB} || {CTRL} && {UP} || {CTRL} && {DOWN}"),
            save_all: format!("{CTRL} && s"),
            cancel: ESC.to_owned(),
            find: format!("{CTRL} && f"),
            replace: format!("{CTRL} && h"),
            backspace_tree_input: BACKSPACE.to_owned(),
            exit: format!("{CTRL} && q || {CTRL} && d"),
            hide_file_tree: format!("{CTRL} && e"),
            refresh_settings: format!("{F}5"),
            go_to_line: format!("{CTRL} && g"),
            git_tui: format!("{CTRL} && {SHIFT} && g"),
            toggle_terminal: format!("{CTRL} && `"),
            context_menu: MENU.to_owned(),
            go_to_tab_1: format!("{ALT} && 1"),
            go_to_tab_2: format!("{ALT} && 2"),
            go_to_tab_3: format!("{ALT} && 3"),
            go_to_tab_4: format!("{ALT} && 4"),
            go_to_tab_5: format!("{ALT} && 5"),
            go_to_tab_6: format!("{ALT} && 6"),
            go_to_tab_7: format!("{ALT} && 7"),
            go_to_tab_8: format!("{ALT} && 8"),
            go_to_tab_9: format!("{ALT} && 9"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TreeAction {
    Up,
    Down,
    Expand,
    Shrink,
    Delete,
    Rename,
    NewFile,
    CopyPath,
    CopyPathRelative,
    CopyFile,
    MarkCopyFile,
    CutFile,
    MarkCutFile,
    Paste,
    IncreaseSize,
    DecreaseSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TreeUserKeyMap {
    select_up: String,
    select_down: String,
    expand: String,
    shrink: String,
    delete: String,
    rename: String,
    new_file: String,
    copy_path: String,
    copy_path_relative: String,
    copy_file: String,
    mark_copy_file: String,
    cut_file: String,
    mark_cut_file: String,
    paste: String,
    increase_size: String,
    decrease_size: String,
}

impl Default for TreeUserKeyMap {
    fn default() -> Self {
        Self {
            select_up: format!("{UP} || w"),
            select_down: format!("{DOWN} || d"),
            expand: format!("{RIGHT} || d || {ENTER}"),
            shrink: format!("{LEFT} || a"),
            delete: format!("{SHIFT} && {DELETE}"),
            rename: format!("{F}2"),
            new_file: format!("{CTRL} && n"),
            copy_path: format!("{ALT} && c"),
            copy_path_relative: format!("{ALT} && {SHIFT} && c"),
            copy_file: format!("{CTRL} && c || {CTRL} && {SHIFT} && c"),
            mark_copy_file: String::from('c'),
            cut_file: format!("{CTRL} && x"),
            mark_cut_file: String::from('x'),
            paste: format!("{CTRL} && v"),
            increase_size: format!("{CTRL} && {RIGHT}"),
            decrease_size: format!("{CTRL} && {LEFT}"),
        }
    }
}

impl From<TreeUserKeyMap> for HashMap<KeyEvent, TreeAction> {
    fn from(val: TreeUserKeyMap) -> Self {
        let mut hash = HashMap::default();
        insert_key_event(&mut hash, &val.select_up, TreeAction::Up);
        insert_key_event(&mut hash, &val.select_down, TreeAction::Down);
        insert_key_event(&mut hash, &val.expand, TreeAction::Expand);
        insert_key_event(&mut hash, &val.shrink, TreeAction::Shrink);
        insert_key_event(&mut hash, &val.delete, TreeAction::Delete);
        insert_key_event(&mut hash, &val.rename, TreeAction::Rename);
        insert_key_event(&mut hash, &val.new_file, TreeAction::NewFile);
        insert_key_event(&mut hash, &val.copy_path_relative, TreeAction::CopyPathRelative);
        insert_key_event(&mut hash, &val.copy_path, TreeAction::CopyPath);
        insert_key_event(&mut hash, &val.copy_file, TreeAction::CopyFile);
        insert_key_event(&mut hash, &val.mark_copy_file, TreeAction::MarkCopyFile);
        insert_key_event(&mut hash, &val.cut_file, TreeAction::CutFile);
        insert_key_event(&mut hash, &val.mark_cut_file, TreeAction::MarkCutFile);
        insert_key_event(&mut hash, &val.paste, TreeAction::Paste);
        insert_key_event(&mut hash, &val.increase_size, TreeAction::IncreaseSize);
        insert_key_event(&mut hash, &val.decrease_size, TreeAction::DecreaseSize);
        hash
    }
}

// SUPPORT functions
fn parse_key(keys: &str) -> KeyEvent {
    let mut modifier = KeyModifiers::NONE;
    let mut code = None;
    for key in keys.split("&&") {
        let trimmed = key.trim();
        if trimmed.len() == 1 {
            if let Some(ch) = trimmed.chars().next() {
                code.replace(KeyCode::Char(ch));
                continue;
            }
        }
        let trimmed_case_indif = key.trim().to_lowercase();
        match trimmed_case_indif.as_str() {
            BACKSPACE => replace_option(&mut code, KeyCode::Backspace),
            ENTER => replace_option(&mut code, KeyCode::Enter),
            LEFT => replace_option(&mut code, KeyCode::Left),
            RIGHT => replace_option(&mut code, KeyCode::Right),
            UP => replace_option(&mut code, KeyCode::Up),
            DOWN => replace_option(&mut code, KeyCode::Down),
            HOME => replace_option(&mut code, KeyCode::Home),
            END => replace_option(&mut code, KeyCode::End),
            PAGEUP => replace_option(&mut code, KeyCode::PageUp),
            PAGEDOWN => replace_option(&mut code, KeyCode::PageDown),
            TAB => replace_option(&mut code, KeyCode::Tab),
            DELETE => replace_option(&mut code, KeyCode::Delete),
            INSERT => replace_option(&mut code, KeyCode::Insert),
            ESC => replace_option(&mut code, KeyCode::Esc),
            BACKTAB => replace_option(&mut code, KeyCode::BackTab),
            MENU => replace_option(&mut code, KeyCode::Menu),

            SHIFT => modifier.toggle(KeyModifiers::SHIFT),
            CTRL => modifier.toggle(KeyModifiers::CONTROL),
            ALT => modifier.toggle(KeyModifiers::ALT),
            META => modifier.toggle(KeyModifiers::META),
            HYPER => modifier.toggle(KeyModifiers::HYPER),
            SUPER => modifier.toggle(KeyModifiers::SUPER),
            _ => {}
        }
        if trimmed_case_indif.starts_with(F) {
            let (_, serialized_value) = trimmed_case_indif.split_at(1);
            if let Ok(f_value) = serialized_value.parse::<u8>() {
                if f_value <= 12 {
                    replace_option(&mut code, KeyCode::F(f_value))
                }
            }
        }
    }
    let mut key_event = KeyEvent::new(code.unwrap_or(KeyCode::Null), modifier);
    if key_event.code == KeyCode::BackTab {
        key_event.modifiers.toggle(KeyModifiers::SHIFT)
    }
    if key_event.code == KeyCode::Tab && key_event.modifiers.contains(KeyModifiers::SHIFT) {
        key_event.code = KeyCode::BackTab
    }
    if key_event.code == KeyCode::Char('[') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
        key_event.code = KeyCode::Esc;
        key_event.modifiers.remove(KeyModifiers::CONTROL)
    }
    key_event
}

fn replace_option<T>(code: &mut Option<T>, value: T) {
    code.replace(value);
}

fn split_mod_char_key_event(key: KeyEvent) -> Vec<KeyEvent> {
    let mut events = vec![key];
    if key.modifiers != KeyModifiers::NONE {
        if let KeyCode::Char(ch) = key.code {
            if ch.is_lowercase() {
                if let Some(new_ch) = ch.to_lowercase().next() {
                    if ch != new_ch {
                        events.push(KeyEvent::new(KeyCode::Char(new_ch), key.modifiers))
                    }
                }
            }
            if ch.is_uppercase() {
                if let Some(new_ch) = ch.to_uppercase().next() {
                    if ch != new_ch {
                        events.push(KeyEvent::new(KeyCode::Char(new_ch), key.modifiers))
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char(']')) => events.push(KeyEvent::new(KeyCode::Char('5'), key.modifiers)),
        (KeyModifiers::CONTROL, KeyCode::Char('`')) => events.push(KeyEvent::new(KeyCode::Char(' '), key.modifiers)),
        (KeyModifiers::CONTROL, KeyCode::Char('/')) => events.push(KeyEvent::new(KeyCode::Char('7'), key.modifiers)),
        _ => (),
    }
    events
}

fn insert_key_event<T: Copy>(hash: &mut HashMap<KeyEvent, T>, se_keys: &str, action: T) {
    for serialized_key in se_keys.split("||") {
        let key_events = split_mod_char_key_event(parse_key(serialized_key));
        for key_event in key_events {
            hash.insert(key_event, action);
        }
    }
}
