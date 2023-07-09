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
    JumpRight,
    Cut,
    Copy,
    Paste,
    Refresh,
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
    jump_right: String,
    cut: String,
    copy: String,
    paste: String,
    refresh: String,
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
        insert_key_event(&mut hash, &val.jump_right, EditorAction::JumpRight);
        insert_key_event(&mut hash, &val.cut, EditorAction::Cut);
        insert_key_event(&mut hash, &val.copy, EditorAction::Copy);
        insert_key_event(&mut hash, &val.paste, EditorAction::Paste);
        insert_key_event(&mut hash, &val.refresh, EditorAction::Refresh);
        hash
    }
}

impl EditorUserKeyMap {
    pub fn default_configs() -> Self {
        Self {
            new_line: String::from(ENTER),
            indent: String::from(TAB),
            backspace: String::from(BACKSPACE),
            delete: String::from(DELETE),
            indent_start: format!("{} + {}", CTRL, ']'),
            unindent: format!("{} + {}", CTRL, '['),
            up: String::from(UP),
            down: String::from(DOWN),
            left: String::from(LEFT),
            right: String::from(RIGHT),
            select_up: format!("{} + {}", SHIFT, UP),
            select_down: format!("{} + {}", SHIFT, DOWN),
            select_left: format!("{} + {}", SHIFT, LEFT),
            select_right: format!("{} + {}", SHIFT, RIGHT),
            scroll_up: format!("{} + {}", CTRL, UP),
            scroll_down: format!("{} + {}", CTRL, DOWN),
            swap_up: format!("{} + {}", ALT, UP),
            swap_down: format!("{} + {}", ALT, DOWN),
            jump_left: format!("{} + {}", CTRL, LEFT),
            jump_right: format!("{} + {}", CTRL, RIGHT),
            cut: format!("{} + {}", CTRL, 'x'),
            copy: format!("{} + {}", CTRL, 'c'),
            paste: format!("{} + {}", CTRL, 'v'),
            refresh: format!("{}{}", F, '5'),
        }
    }
}

// TREE

pub enum TreeAction {}

// SUPPORT functions

fn parse_key(keys: &str) -> KeyEvent {
    let mut modifier = KeyModifiers::NONE;
    let mut code = None;
    for key in keys.split('+') {
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
            SHIFT => modifier.toggle(KeyModifiers::SHIFT),
            CTRL => modifier.toggle(KeyModifiers::CONTROL),
            ALT => modifier.toggle(KeyModifiers::ALT),
            META => modifier.toggle(KeyModifiers::META),
            HYPER => modifier.toggle(KeyModifiers::HYPER),
            SUPER => modifier.toggle(KeyModifiers::SUPER),
            _ => {}
        }
    }
    KeyEvent::new(code.unwrap_or(KeyCode::Null), modifier)
}

fn replace_option<T>(key_code: &mut Option<T>, value: T) {
    key_code.replace(value);
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
    events
}

fn insert_key_event<T: Copy>(hash: &mut HashMap<KeyEvent, T>, serialized_key: &str, action: T) {
    let key_events = split_mod_char_key_event(parse_key(serialized_key));
    for key_event in key_events {
        hash.insert(key_event, action);
    }
}
