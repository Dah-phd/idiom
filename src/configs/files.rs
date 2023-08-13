use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

// MODS
const SHIFT: &str = "shift";
const CTRL: &str = "ctrl";
const ALT: &str = "alt";
const SUPER: &str = "super";
const HYPER: &str = "hyper";
const META: &str = "meta";

// KEYS
const BACKSPACE: &str = "backspace";
const ENTER: &str = "enter";
const LEFT: &str = "left";
const RIGHT: &str = "right";
const UP: &str = "up";
const DOWN: &str = "down";
const HOME: &str = "home";
const END: &str = "end";
const PAGEUP: &str = "pageup";
const PAGEDOWN: &str = "pagedown";
const TAB: &str = "tab";
const BACKTAB: &str = "backtab";
const DELETE: &str = "delete";
const INSERT: &str = "insert";
const F: &str = "f";
const ESC: &str = "esc";

// EDITOR

#[derive(Debug, Clone, Copy)]
pub enum EditorAction {
    Char(char),
    NewLine,
    Indent,
    Backspace,
    Delete,
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
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    Save,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorUserKeyMap {
    new_line: String,
    indent: String,
    backspace: String,
    delete: String,
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
    scroll_up: String,
    scroll_down: String,
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
    cut: String,
    copy: String,
    paste: String,
    undo: String,
    redo: String,
    save: String,
    close: String,
}

impl From<EditorUserKeyMap> for HashMap<KeyEvent, EditorAction> {
    fn from(val: EditorUserKeyMap) -> Self {
        let mut hash = HashMap::default();
        insert_key_event(&mut hash, &val.new_line, EditorAction::NewLine);
        insert_key_event(&mut hash, &val.indent, EditorAction::Indent);
        insert_key_event(&mut hash, &val.backspace, EditorAction::Backspace);
        insert_key_event(&mut hash, &val.delete, EditorAction::Delete);
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
        insert_key_event(&mut hash, &val.cut, EditorAction::Cut);
        insert_key_event(&mut hash, &val.copy, EditorAction::Copy);
        insert_key_event(&mut hash, &val.paste, EditorAction::Paste);
        insert_key_event(&mut hash, &val.undo, EditorAction::Undo);
        insert_key_event(&mut hash, &val.redo, EditorAction::Redo);
        insert_key_event(&mut hash, &val.save, EditorAction::Save);
        insert_key_event(&mut hash, &val.close, EditorAction::Close);
        hash
    }
}

impl Default for EditorUserKeyMap {
    fn default() -> Self {
        Self {
            new_line: String::from(ENTER),
            indent: String::from(TAB),
            backspace: String::from(BACKSPACE),
            delete: String::from(DELETE),
            indent_start: format!("{} && {}", CTRL, ']'),
            unindent: format!("{} && {}", SHIFT, TAB),
            up: String::from(UP),
            down: String::from(DOWN),
            left: String::from(LEFT),
            right: String::from(RIGHT),
            select_up: format!("{} && {}", SHIFT, UP),
            select_down: format!("{} && {}", SHIFT, DOWN),
            select_left: format!("{} && {}", SHIFT, LEFT),
            select_right: format!("{} && {}", SHIFT, RIGHT),
            scroll_up: format!("{} && {} || {}", CTRL, UP, PAGEUP),
            scroll_down: format!("{} && {} || {}", CTRL, DOWN, PAGEDOWN),
            swap_up: format!("{} && {}", ALT, UP),
            swap_down: format!("{} && {}", ALT, DOWN),
            jump_left: format!("{} && {}", CTRL, LEFT),
            jump_left_select: format!("{} && {} && {}", CTRL, SHIFT, LEFT),
            jump_right: format!("{} && {}", CTRL, RIGHT),
            jump_right_select: format!("{} && {} && {}", CTRL, SHIFT, RIGHT),
            end_of_line: String::from(END),
            end_of_file: format!("{} && {}", CTRL, END),
            start_of_line: String::from(HOME),
            start_of_file: format!("{} && {}", CTRL, HOME),
            cut: format!("{} && {}", CTRL, 'x'),
            copy: format!("{} && {}", CTRL, 'c'),
            paste: format!("{} && {}", CTRL, 'v'),
            undo: format!("{} && {}", CTRL, 'z'),
            redo: format!("{} && {}", CTRL, 'y'),
            save: format!("{} && {}", CTRL, 's'),
            close: format!("{} && {} || {} && {}", CTRL, 'q', CTRL, 'd'),
        }
    }
}

// TREE

#[derive(Debug, Clone, Copy)]
pub enum GeneralAction {
    Char(char),
    Up,
    Down,
    Shrink,
    Expand,
    FinishOrSelect,
    SaveAll,
    FileTreeModeOrCancelInput,
    NewFile,
    DeleteFile,
    BackspaceTreeInput,
    Exit,
    HideFileTree,
    NextTab,
    PreviousTab,
    RefreshSettings,
    GoToLinePopup,
    ToggleTerminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralUserKeyMap {
    up_file_tree: String,
    down_file_tree: String,
    shrink_path: String,
    expand_file_tree_or_open_file: String,
    finish_or_select: String,
    save_all: String,
    file_tree_mod_or_cancel_input: String,
    new_file: String,
    delete_file: String,
    backspace_tree_input: String,
    exit: String,
    hide_file_tree: String,
    next_tab: String,
    previous_tab: String,
    refresh_settings: String,
    go_to_line: String,
    toggle_terminal: String,
}

impl From<GeneralUserKeyMap> for HashMap<KeyEvent, GeneralAction> {
    fn from(val: GeneralUserKeyMap) -> Self {
        let mut hash = HashMap::default();
        insert_key_event(&mut hash, &val.up_file_tree, GeneralAction::Up);
        insert_key_event(&mut hash, &val.down_file_tree, GeneralAction::Down);
        insert_key_event(&mut hash, &val.shrink_path, GeneralAction::Shrink);
        insert_key_event(&mut hash, &val.expand_file_tree_or_open_file, GeneralAction::Expand);
        insert_key_event(&mut hash, &val.finish_or_select, GeneralAction::FinishOrSelect);
        insert_key_event(&mut hash, &val.save_all, GeneralAction::SaveAll);
        insert_key_event(
            &mut hash,
            &val.file_tree_mod_or_cancel_input,
            GeneralAction::FileTreeModeOrCancelInput,
        );
        insert_key_event(&mut hash, &val.new_file, GeneralAction::NewFile);
        insert_key_event(&mut hash, &val.delete_file, GeneralAction::DeleteFile);
        insert_key_event(&mut hash, &val.backspace_tree_input, GeneralAction::BackspaceTreeInput);
        insert_key_event(&mut hash, &val.exit, GeneralAction::Exit);
        insert_key_event(&mut hash, &val.hide_file_tree, GeneralAction::HideFileTree);
        insert_key_event(&mut hash, &val.next_tab, GeneralAction::NextTab);
        insert_key_event(&mut hash, &val.previous_tab, GeneralAction::PreviousTab);
        insert_key_event(&mut hash, &val.refresh_settings, GeneralAction::RefreshSettings);
        insert_key_event(&mut hash, &val.go_to_line, GeneralAction::GoToLinePopup);
        insert_key_event(&mut hash, &val.toggle_terminal, GeneralAction::ToggleTerminal);
        hash
    }
}

impl Default for GeneralUserKeyMap {
    fn default() -> Self {
        Self {
            up_file_tree: format!("{} || {} || {}", UP, 'w', 'W'),
            down_file_tree: format!("{} || {} || {}", DOWN, 's', 'S'),
            shrink_path: format!("{} || {} || {}", LEFT, 'a', 'A'),
            expand_file_tree_or_open_file: format!("{} || {} || {}", RIGHT, 'd', 'D'),
            finish_or_select: String::from(ENTER),
            save_all: format!("{} && {}", CTRL, 's'),
            file_tree_mod_or_cancel_input: String::from(ESC),
            new_file: format!("{} && {}", CTRL, 'n'),
            delete_file: format!("{} && {}", SHIFT, DELETE),
            backspace_tree_input: String::from(BACKSPACE),
            exit: format!("{} && {} || {} && {}", CTRL, 'd', CTRL, 'q'),
            hide_file_tree: format!("{} && {}", CTRL, 'e'),
            next_tab: String::from(TAB),
            previous_tab: format!("{} && {}", CTRL, TAB),
            refresh_settings: format!("{}5", F),
            go_to_line: format!("{} && {}", CTRL, 'g'),
            toggle_terminal: format!("{} && {}", CTRL, '`'),
        }
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
