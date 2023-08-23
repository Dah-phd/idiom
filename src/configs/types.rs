use tui::{backend::Backend, Frame};

use crate::components::popups::Popup;
use std::path::PathBuf;

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
    pub fn render_popup_if_exists(&mut self, frame: &mut Frame<impl Backend>) {
        if let Self::Popup((.., popup)) = self {
            popup.render(frame)
        }
    }

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
