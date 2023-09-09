use crossterm::event::KeyEvent;
use tui::{backend::CrosstermBackend, Frame};

use crate::components::popups::PopupInterface;
use std::{io::Stdout, path::PathBuf};

use super::PopupMessage;

pub enum Mode {
    Select,
    Insert,
    Popup((Box<Mode>, Box<dyn PopupInterface>)),
}

impl Mode {
    pub fn render_popup_if_exists(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        if let Self::Popup((.., popup)) = self {
            popup.render(frame)
        }
    }

    pub fn popup_map(&mut self, key: &KeyEvent) -> Option<PopupMessage> {
        if let Self::Popup((.., popup)) = self {
            return Some(popup.map(key));
        }
        None
    }

    pub fn popup(self, popup: Box<dyn PopupInterface>) -> Self {
        if matches!(self, Self::Popup((_, _))) {
            return self;
        }
        Self::Popup((Box::new(self), popup))
    }

    pub fn clear_popup(self) -> Self {
        if let Self::Popup((mode, _)) = self {
            return *mode;
        }
        self
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Html,
    C,
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

impl From<&FileType> for String {
    fn from(value: &FileType) -> String {
        match value {
            FileType::Rust => "rust",
            FileType::Python => "python",
            FileType::TypeScript => "typescript",
            FileType::JavaScript => "javascript",
            FileType::Html => "html",
            FileType::C => "c",
            FileType::Yml => "yaml",
            FileType::Toml => "toml",
            _ => "unknown",
        }
        .to_owned()
    }
}
