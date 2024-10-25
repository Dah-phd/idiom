use super::defaults::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// EDITOR
#[derive(Debug, Clone, Copy)]
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
pub struct EditorUserKeyMap {
    #[serde(default = "new_line")]
    new_line_or_select: String,
    #[serde(default = "tab")]
    indent: String,
    #[serde(default = "backspace")]
    backspace: String,
    #[serde(default = "delete")]
    delete: String,
    #[serde(default = "remove_line")]
    remove_line: String,
    #[serde(default = "indent_start")]
    indent_start: String,
    #[serde(default = "unindent")]
    unindent: String,
    #[serde(default = "up")]
    up: String,
    #[serde(default = "down")]
    down: String,
    #[serde(default = "left")]
    left: String,
    #[serde(default = "right")]
    right: String,
    #[serde(default = "select_up")]
    select_up: String,
    #[serde(default = "select_down")]
    select_down: String,
    #[serde(default = "select_left")]
    select_left: String,
    #[serde(default = "select_right")]
    select_right: String,
    #[serde(default = "select_token")]
    select_token: String,
    #[serde(default = "select_line")]
    select_line: String,
    #[serde(default = "select_all")]
    select_all: String,
    #[serde(default = "scroll_up")]
    scroll_up: String,
    #[serde(default = "scroll_down")]
    scroll_down: String,
    #[serde(default = "swap_up")]
    swap_up: String,
    #[serde(default = "swap_down")]
    swap_down: String,
    #[serde(default = "jump_left")]
    jump_left: String,
    #[serde(default = "jump_left_select")]
    jump_left_select: String,
    #[serde(default = "jump_right")]
    jump_right: String,
    #[serde(default = "jump_right_select")]
    jump_right_select: String,
    #[serde(default = "end")]
    end_of_line: String,
    #[serde(default = "end_of_file")]
    end_of_file: String,
    #[serde(default = "home")]
    start_of_line: String,
    #[serde(default = "start_of_file")]
    start_of_file: String,
    #[serde(default = "find_references")]
    find_references: String,
    #[serde(default = "go_to_declaration")]
    go_to_declaration: String,
    #[serde(default = "help")]
    help: String,
    #[serde(default = "refresh")]
    refresh_ui: String,
    #[serde(default = "lsp_rename")]
    lsp_rename: String,
    #[serde(default = "cut")]
    cut: String,
    #[serde(default = "copy")]
    copy: String,
    #[serde(default = "paste")]
    paste: String,
    #[serde(default = "undo")]
    undo: String,
    #[serde(default = "redo")]
    redo: String,
    #[serde(default = "save")]
    save: String,
    #[serde(default = "esc")]
    cancel: String,
    #[serde(default = "close")]
    close: String,
    #[serde(default = "comment_out")]
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
        insert_key_event(&mut hash, &val.scroll_down, EditorAction::ScrollDown);
        insert_key_event(&mut hash, &val.swap_up, EditorAction::SwapUp);
        insert_key_event(&mut hash, &val.swap_down, EditorAction::SwapDown);
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
            new_line_or_select: new_line(),
            indent: tab(),
            backspace: backspace(),
            delete: delete(),
            remove_line: remove_line(),
            indent_start: indent_start(),
            unindent: unindent(),
            up: up(),
            down: down(),
            left: left(),
            right: right(),
            select_up: select_up(),
            select_down: select_down(),
            select_left: select_left(),
            select_right: select_right(),
            select_token: select_token(),
            select_line: select_line(),
            select_all: select_all(),
            scroll_up: scroll_up(),
            scroll_down: scroll_down(),
            swap_up: swap_up(),
            swap_down: swap_down(),
            jump_left: jump_left(),
            jump_left_select: jump_left_select(),
            jump_right: jump_right(),
            jump_right_select: jump_right_select(),
            end_of_line: end(),
            end_of_file: end_of_file(),
            start_of_line: home(),
            start_of_file: start_of_file(),
            find_references: find_references(),
            go_to_declaration: go_to_declaration(),
            help: help(),
            refresh_ui: refresh(),
            lsp_rename: lsp_rename(),
            cut: cut(),
            copy: copy(),
            paste: paste(),
            undo: undo(),
            redo: redo(),
            save: save(),
            cancel: esc(),
            close: close(),
            comment_out: comment_out(),
        }
    }
}

// TREE

#[derive(Debug, Clone, Copy)]
pub enum GeneralAction {
    GoToTabs,
    SelectOpenEditor,
    SaveAll,
    FileTreeModeOrCancelInput,
    Find,
    Replace,
    Exit,
    HideFileTree,
    RefreshSettings,
    GoToLinePopup,
    ToggleTerminal,
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
pub struct GeneralUserKeyMap {
    #[serde(default = "tab")]
    go_to_editor_tabs: String,
    #[serde(default = "select_open_editor")]
    select_open_editor: String,
    #[serde(default = "save")]
    save_all: String,
    #[serde(default = "esc")]
    cancel: String,
    #[serde(default = "find")]
    find: String,
    #[serde(default = "replace")]
    replace: String,
    #[serde(default = "backspace")]
    backspace_tree_input: String,
    #[serde(default = "close")]
    exit: String,
    #[serde(default = "hide_file_tree")]
    hide_file_tree: String,
    #[serde(default = "refresh")]
    refresh_settings: String,
    #[serde(default = "go_to")]
    go_to_line: String,
    #[serde(default = "terminal")]
    toggle_terminal: String,
    #[serde(default = "tab1")]
    go_to_tab_1: String,
    #[serde(default = "tab2")]
    go_to_tab_2: String,
    #[serde(default = "tab3")]
    go_to_tab_3: String,
    #[serde(default = "tab4")]
    go_to_tab_4: String,
    #[serde(default = "tab5")]
    go_to_tab_5: String,
    #[serde(default = "tab6")]
    go_to_tab_6: String,
    #[serde(default = "tab7")]
    go_to_tab_7: String,
    #[serde(default = "tab8")]
    go_to_tab_8: String,
    #[serde(default = "tab9")]
    go_to_tab_9: String,
}

impl From<GeneralUserKeyMap> for HashMap<KeyEvent, GeneralAction> {
    fn from(val: GeneralUserKeyMap) -> Self {
        let mut hash = HashMap::default();
        insert_key_event(&mut hash, &val.go_to_editor_tabs, GeneralAction::GoToTabs);
        insert_key_event(&mut hash, &val.select_open_editor, GeneralAction::SelectOpenEditor);
        insert_key_event(&mut hash, &val.save_all, GeneralAction::SaveAll);
        insert_key_event(&mut hash, &val.cancel, GeneralAction::FileTreeModeOrCancelInput);
        insert_key_event(&mut hash, &val.find, GeneralAction::Find);
        insert_key_event(&mut hash, &val.replace, GeneralAction::Replace);
        insert_key_event(&mut hash, &val.exit, GeneralAction::Exit);
        insert_key_event(&mut hash, &val.hide_file_tree, GeneralAction::HideFileTree);
        insert_key_event(&mut hash, &val.refresh_settings, GeneralAction::RefreshSettings);
        insert_key_event(&mut hash, &val.go_to_line, GeneralAction::GoToLinePopup);
        insert_key_event(&mut hash, &val.toggle_terminal, GeneralAction::ToggleTerminal);
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
            go_to_editor_tabs: tab(),
            select_open_editor: select_open_editor(),
            save_all: save(),
            cancel: esc(),
            find: find(),
            replace: replace(),
            backspace_tree_input: backspace(),
            exit: close(),
            hide_file_tree: hide_file_tree(),
            refresh_settings: refresh(),
            go_to_line: go_to(),
            toggle_terminal: terminal(),
            go_to_tab_1: tab1(),
            go_to_tab_2: tab2(),
            go_to_tab_3: tab3(),
            go_to_tab_4: tab4(),
            go_to_tab_5: tab5(),
            go_to_tab_6: tab6(),
            go_to_tab_7: tab7(),
            go_to_tab_8: tab8(),
            go_to_tab_9: tab9(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TreeAction {
    Up,
    Down,
    Expand,
    Shrink,
    Delete,
    Rename,
    NewFile,
    IncreaseSize,
    DecreaseSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeUserKeyMap {
    select_up: String,
    select_down: String,
    expand: String,
    shrink: String,
    delete: String,
    rename: String,
    new_file: String,
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
