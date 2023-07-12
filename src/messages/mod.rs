use crate::components::popups::Popup;
use std::path::PathBuf;

mod action_map;
mod configs;
mod popups;
pub use action_map::{EditorAction, GeneralAction};
pub use configs::{load_or_create_config, EditorConfigs, EditorKeyMap, KeyMap};
pub use popups::PopupMessage;

#[derive(Debug, Clone)]
pub enum Mode {
    Select,
    Insert,
    Popup((Box<Mode>, Popup)),
}

impl Default for Mode {
    fn default() -> Self {
        Self::Select
    }
}

impl Mode {
    pub fn popup(self, popup: Popup) -> Self {
        if matches!(self, Self::Popup((_, _))) {
            return self;
        }
        Self::Popup((Box::new(self), popup))
    }

    pub fn clear_popup(&mut self) {
        if let Self::Popup((mode, _)) = self {
            (*self) = *mode.clone();
        }
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    Html,
    Yml,
    Toml,
    Unknown,
}

impl FileType {
    #[allow(clippy::ptr_arg)]
    pub fn derive_type(path: &PathBuf) -> Self {
        if let Some(extension_os_str) = path.extension() {
            if let Some(extension) = extension_os_str.to_str() {
                return match extension.to_lowercase().as_str() {
                    "rs" => Self::Rust,
                    "py" | "pyw" => Self::Python,
                    "js" => Self::JavaScript,
                    "yml" | "yaml" => Self::Yml,
                    "toml" => Self::Toml,
                    "html" => Self::Html,
                    _ => Self::Unknown,
                };
            };
        };
        Self::Unknown
    }
}
