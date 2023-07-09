use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

use super::action_map::{EditorAction, EditorUserKeyMap};

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
                    write_config_file(EDITOR_CONFIGS, &Self::default_configs());
                    Self::default_configs()
                }
            }
        } else {
            write_config_file(EDITOR_CONFIGS, &Self::default_configs());
            Self::default_configs()
        }
    }
}

impl EditorConfigs {
    fn default_configs() -> Self {
        Self {
            indent: "    ".to_owned(),
            indent_after: ":({".to_owned(),
            format_on_save: true,
        }
    }
}

impl EditorConfigs {
    pub fn refresh(&mut self) {
        (*self) = Self::default()
    }
}

pub struct EditorKeyMap {
    key_map: HashMap<KeyEvent, EditorAction>,
}

impl EditorKeyMap {
    pub fn map(&self, key: &KeyEvent) -> Option<EditorAction> {
        if let KeyCode::Char(ch) = key.code {
            if key.modifiers == KeyModifiers::NONE {
                return Some(EditorAction::Char(ch));
            }
        }
        if let Some(action) = self.key_map.get(key) {
            return Some(*action);
        }
        None
    }
}

pub struct GeneralKeyMap {}

pub struct TreeKeyMap {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMap {
    // general_key_map: GeneralKeyMap,
    editor_key_map: EditorUserKeyMap,
    // tree_key_map: TreeKeyMap,
}

impl Default for KeyMap {
    fn default() -> Self {
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
                    write_config_file(KEY_MAP, &Self::default_configs());
                    Self::default_configs()
                }
            }
        } else {
            write_config_file(KEY_MAP, &Self::default_configs());
            Self::default_configs()
        }
    }
}

impl KeyMap {
    fn default_configs() -> Self {
        Self {
            editor_key_map: EditorUserKeyMap::default_configs(),
        }
    }

    pub fn editor_key_map(&self) -> EditorKeyMap {
        EditorKeyMap {
            key_map: self.editor_key_map.clone().into(),
        }
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
