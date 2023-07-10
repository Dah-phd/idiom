use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

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
}

impl Default for EditorConfigs {
    fn default() -> Self {
        Self {
            indent: "    ".to_owned(),
            indent_after: ":({".to_owned(),
            format_on_save: true,
        }
    }
}

impl EditorConfigs {
    pub fn new() -> Self {
        if let Some(config_json) = read_config_file(EDITOR_CONFIGS) {
            match serde_json::from_slice::<EditorConfigs>(&config_json) {
                Ok(configs) => configs,
                Err(error) => {
                    match error.classify() {
                        Category::Data => {}
                        Category::Eof => {}
                        Category::Io => {}
                        Category::Syntax => {}
                    };
                    write_config_file(EDITOR_CONFIGS, &Self::default());
                    Self::default()
                }
            }
        } else {
            write_config_file(EDITOR_CONFIGS, &Self::default());
            Self::default()
        }
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
        if let Some(config_json) = read_config_file(KEY_MAP) {
            match serde_json::from_slice::<Self>(&config_json) {
                Ok(configs) => configs,
                Err(error) => {
                    match error.classify() {
                        Category::Data => {}
                        Category::Eof => {}
                        Category::Io => {}
                        Category::Syntax => {}
                    };
                    write_config_file(KEY_MAP, &Self::default());
                    Self::default()
                }
            }
        } else {
            write_config_file(KEY_MAP, &Self::default());
            Self::default()
        }
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

// fn setup_config<T: Deserialize<'de> + Serialize + Default>(path: &str) -> T {
//     if let Some(config_json) = read_config_file(path) {
//         match serde_json::from_slice::<T>(&config_json) {
//             Ok(configs) => configs,
//             Err(error) => {
//                 match error.classify() {
//                     Category::Data => {}
//                     Category::Eof => {}
//                     Category::Io => {}
//                     Category::Syntax => {}
//                 };
//                 write_config_file(KEY_MAP, &T::default());
//                 T::default()
//             }
//         }
//     } else {
//         write_config_file(KEY_MAP, &T::default());
//         T::default()
//     }
// }

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
