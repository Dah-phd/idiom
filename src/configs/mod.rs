mod defaults;
mod editor;
mod keymap;
mod theme;
mod theme_ui;
mod types;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dirs::config_dir;
pub use editor::{EditorConfigs, IndentConfigs};
pub use keymap::{EditorAction, EditorUserKeyMap, GeneralAction, GeneralUserKeyMap, TreeAction, TreeUserKeyMap};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
pub use theme::Theme;
pub use theme_ui::UITheme;
pub use types::{FileFamily, FileType};

pub const APP_FOLDER: &str = "idiom";
pub const EDITOR_CFG_FILE: &str = "editor.toml";
pub const KEY_MAP: &str = "keys.toml";
pub const THEME_FILE: &str = "theme.toml";
pub const THEME_UI: &str = "theme_ui.toml";

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

#[derive(Serialize, Deserialize)]
pub struct KeyMap {
    general_key_map: Option<GeneralUserKeyMap>,
    editor_key_map: Option<EditorUserKeyMap>,
    tree_key_map: Option<TreeUserKeyMap>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            general_key_map: Some(GeneralUserKeyMap::default()),
            editor_key_map: Some(EditorUserKeyMap::default()),
            tree_key_map: Some(TreeUserKeyMap::default()),
        }
    }
}

impl KeyMap {
    pub fn new() -> Result<Self, toml::de::Error> {
        load_or_create_config(KEY_MAP)
    }

    pub fn unpack(self) -> (GeneralKeyMap, EditorKeyMap, TreeKeyMap) {
        let Self { general_key_map, editor_key_map, tree_key_map } = self;
        (
            GeneralKeyMap { key_map: general_key_map.unwrap_or_default().into() },
            EditorKeyMap { key_map: editor_key_map.unwrap_or_default().into() },
            TreeKeyMap { key_map: tree_key_map.unwrap_or_default().into() },
        )
    }

    pub fn tree_key_map(self) -> TreeKeyMap {
        TreeKeyMap { key_map: self.tree_key_map.unwrap_or_default().into() }
    }
}

/// ensures creation of config files on first load
/// if value is removed from a theme config the default value will be put in place
pub fn load_or_create_config<T: Default + DeserializeOwned + Serialize>(path: &str) -> Result<T, toml::de::Error> {
    if let Some(config_json) = read_config_file(path) {
        Ok(toml::from_str::<T>(&config_json)?)
    } else {
        write_config_file(path, &T::default());
        Ok(T::default())
    }
}

/// should not fail as config files/dirs are created on start
pub fn get_config_dir() -> Option<PathBuf> {
    let mut config_path = config_dir()?;
    config_path.push(APP_FOLDER);
    Some(config_path)
}

fn read_config_file(path: &str) -> Option<String> {
    let mut config_file = config_dir()?;
    config_file.push(APP_FOLDER);
    config_file.push(path);
    std::fs::read_to_string(config_file).ok()
}

fn write_config_file<T: Serialize>(path: &str, configs: &T) -> Option<()> {
    let mut config_file = config_dir()?;
    config_file.push(APP_FOLDER);
    if !config_file.exists() {
        std::fs::create_dir(&config_file).ok()?;
    }
    config_file.push(path);
    let serialized = toml::to_string_pretty(configs).ok()?;
    std::fs::write(config_file, serialized).ok()
}

#[cfg(test)]
pub mod test;
