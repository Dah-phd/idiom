use std::path::PathBuf;
mod editor_config;
mod keymap;
mod theme_ui;
mod types;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dirs::config_dir;
pub use editor_config::{EditorConfigs, IndentConfigs};
pub use keymap::{EditorAction, EditorUserKeyMap, GeneralAction, GeneralUserKeyMap, TreeAction, TreeUserKeyMap};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Error;
use std::collections::HashMap;
pub use theme_ui::{pull_color, serialize_rgb, UITheme};
pub use types::FileType;

pub const CONFIG_FOLDER: &str = "idiom";
pub const EDITOR_CFG_FILE: &str = ".editor";
pub const KEY_MAP: &str = ".keys";
pub const THEME_FILE: &str = "theme.json";
pub const THEME_UI: &str = "theme_ui.json";

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
        self.key_map.get(key).copied()
    }
}

pub struct TreeKeyMap {
    key_map: HashMap<KeyEvent, TreeAction>,
}

impl TreeKeyMap {
    pub fn map(&self, key: &KeyEvent) -> Option<TreeAction> {
        self.key_map.get(key).copied()
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyMap {
    general_key_map: GeneralUserKeyMap,
    editor_key_map: EditorUserKeyMap,
    tree_key_map: TreeUserKeyMap,
}

impl KeyMap {
    pub fn new() -> Result<Self, Error> {
        load_or_create_config(KEY_MAP)
    }

    pub fn editor_key_map(&self) -> EditorKeyMap {
        EditorKeyMap { key_map: self.editor_key_map.clone().into() }
    }

    pub fn general_key_map(&self) -> GeneralKeyMap {
        GeneralKeyMap { key_map: self.general_key_map.clone().into() }
    }

    pub fn tree_key_map(&self) -> TreeKeyMap {
        TreeKeyMap { key_map: self.tree_key_map.clone().into() }
    }
}

pub fn load_or_create_config<T: Default + DeserializeOwned + Serialize>(path: &str) -> Result<T, Error> {
    if let Some(config_json) = read_config_file(path) {
        Ok(serde_json::from_slice::<T>(&config_json)?)
    } else {
        write_config_file(path, &T::default());
        Ok(T::default())
    }
}

/// should not fail as config files/dirs are created on start
pub fn get_config_dir() -> Option<PathBuf> {
    let mut config_path = config_dir()?;
    config_path.push(CONFIG_FOLDER);
    Some(config_path)
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

#[cfg(test)]
pub mod test;
