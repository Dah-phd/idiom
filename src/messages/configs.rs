use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dirs::config_dir;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::error::Category;

use crate::syntax::DEFAULT_THEME_FILE;

use super::action_map::{EditorAction, EditorUserKeyMap, GeneralAction, GeneralUserKeyMap};

const CONFIG_FOLDER: &str = "idiom";
const EDITOR_CONFIGS: &str = ".editor";
const KEY_MAP: &str = ".keys";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigs {
    pub indent: String,
    pub indent_after: String,
    pub format_on_save: bool,
    pub theme_file_in_config_dir: String,
}

impl Default for EditorConfigs {
    fn default() -> Self {
        Self {
            indent: "    ".to_owned(),
            indent_after: "({[".to_owned(),
            format_on_save: true,
            theme_file_in_config_dir: String::from(DEFAULT_THEME_FILE),
        }
    }
}

impl EditorConfigs {
    pub fn new() -> Self {
        load_or_create_config(EDITOR_CONFIGS)
    }

    pub fn derive_indent_from(&self, prev_line: &str) -> (usize, String) {
        let mut indent = prev_line.chars().take_while(|&c| c.is_whitespace()).collect::<String>();
        let mut indent_offset = indent.len();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if self.indent_after.contains(last) {
                indent.insert_str(0, &self.indent);
                indent_offset += &self.indent.len();
            }
        };
        (indent_offset, indent)
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
