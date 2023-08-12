mod files;
mod popups;
mod types;
pub use files::{EditorAction, EditorUserKeyMap, GeneralAction, GeneralUserKeyMap};
pub use types::{FileType, Mode};

pub use popups::PopupMessage;

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dirs::config_dir;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::error::Category;

use crate::{components::editor::Offset, syntax::DEFAULT_THEME_FILE, utils::trim_start_inplace};

const CONFIG_FOLDER: &str = "idiom";
const EDITOR_CONFIGS: &str = ".editor";
const KEY_MAP: &str = ".keys";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigs {
    pub indent: String,
    #[serde(skip, default = "get_indent_after")]
    pub indent_after: String,
    #[serde(skip, default = "get_unident_before")]
    pub unindent_before: String,
    pub format_on_save: bool,
    pub theme_file_in_config_dir: String,
}

impl Default for EditorConfigs {
    fn default() -> Self {
        Self {
            indent: "    ".to_owned(),
            indent_after: get_indent_after(),
            unindent_before: get_unident_before(),
            format_on_save: true,
            theme_file_in_config_dir: String::from(DEFAULT_THEME_FILE),
        }
    }
}

impl EditorConfigs {
    pub fn new() -> Self {
        load_or_create_config(EDITOR_CONFIGS)
    }

    pub fn update_by_file_type(&mut self, file_type: &FileType) {
        #[allow(clippy::single_match)]
        match file_type {
            FileType::Python => self.indent_after.push(':'),
            _ => (),
        }
    }

    pub fn backspace_indent_handler(&self, line: &mut String, from_idx: usize) -> Offset {
        //! does not handle from_idx == 0
        let prefix = line[..from_idx].trim_start_matches(&self.indent);
        if prefix.is_empty() {
            line.replace_range(..self.indent.len(), "");
            return Offset::Neg(self.indent.len());
        }
        if prefix.chars().all(|c| c.is_whitespace()) {
            let remove_chars_len = prefix.len();
            line.replace_range(from_idx - remove_chars_len..from_idx, "");
            return Offset::Neg(remove_chars_len);
        }
        line.remove(from_idx - 1);
        Offset::Neg(1)
    }

    pub fn derive_indent_from(&self, prev_line: &str) -> String {
        let mut indent = prev_line.chars().take_while(|&c| c.is_whitespace()).collect::<String>();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if self.indent_after.contains(last) {
                indent.insert_str(0, &self.indent);
            }
        };
        indent
    }

    pub fn indent_from_prev(&self, prev_line: &str, line: &mut String) -> Offset {
        let indent = self.derive_indent_from(prev_line);
        let offset = trim_start_inplace(line) + indent.len();
        line.insert_str(0, &indent);
        offset + self.unindent_if_before_base_pattern(line)
    }

    pub fn unindent_if_before_base_pattern(&self, line: &mut String) -> Offset {
        if line.starts_with(&self.indent) {
            if let Some(first) = line.trim_start().chars().next() {
                if self.unindent_before.contains(first) {
                    line.replace_range(..self.indent.len(), "");
                    return Offset::Neg(self.indent.len());
                }
            }
        }
        Offset::Pos(0)
    }
}

impl EditorConfigs {
    pub fn refresh(&mut self) {
        (*self) = Self::new()
    }
}

#[derive(Debug)]
pub struct EditorKeyMap {
    key_map: HashMap<KeyEvent, EditorAction>,
}

impl EditorKeyMap {
    pub fn map(&self, key: &KeyEvent) -> Option<EditorAction> {
        if let KeyCode::Char(ch) = key.code {
            if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT {
                return Some(EditorAction::Char(ch));
            }
        }
        self.key_map.get(key).copied()
    }
}

pub struct GeneralKeyMap {
    key_map: HashMap<KeyEvent, GeneralAction>,
}

impl GeneralKeyMap {
    pub fn map(&self, key: &KeyEvent) -> Option<GeneralAction> {
        if let KeyCode::Char(ch) = key.code {
            if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT {
                return Some(GeneralAction::Char(ch));
            }
        }
        self.key_map.get(key).copied()
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyMap {
    general_key_map: GeneralUserKeyMap,
    editor_key_map: EditorUserKeyMap,
}

impl KeyMap {
    pub fn new() -> Self {
        load_or_create_config(KEY_MAP)
    }

    pub fn editor_key_map(&self) -> EditorKeyMap {
        EditorKeyMap {
            key_map: self.editor_key_map.clone().into(),
        }
    }

    pub fn general_key_map(&self) -> GeneralKeyMap {
        GeneralKeyMap {
            key_map: self.general_key_map.clone().into(),
        }
    }
}

pub fn load_or_create_config<T: Default + DeserializeOwned + Serialize>(path: &str) -> T {
    if let Some(config_json) = read_config_file(path) {
        match serde_json::from_slice::<T>(&config_json) {
            Ok(configs) => configs,
            Err(error) => {
                match error.classify() {
                    Category::Data => {}
                    Category::Eof => {}
                    Category::Io => {}
                    Category::Syntax => {}
                };
                write_config_file(path, &T::default());
                T::default()
            }
        }
    } else {
        write_config_file(path, &T::default());
        T::default()
    }
}

fn read_config_file(path: &str) -> Option<Vec<u8>> {
    let mut config_file = config_dir()?;
    config_file.push(CONFIG_FOLDER);
    config_file.push(path);
    std::fs::read(config_file).ok()
}

fn write_config_file<T: Serialize>(path: &str, configs: &T) -> Option<()> {
    let mut config_file = config_dir()?;
    config_file.push(CONFIG_FOLDER);
    if !config_file.exists() {
        std::fs::create_dir(&config_file).ok()?;
    }
    config_file.push(path);
    let serialized = serde_json::to_string_pretty(configs).ok()?;
    std::fs::write(config_file, serialized).ok()
}

fn get_indent_after() -> String {
    String::from("({[")
}

fn get_unident_before() -> String {
    String::from("]})")
}
