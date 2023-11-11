use crossterm::event::KeyEvent;
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::Span,
    Frame,
};

use crate::components::{popups::PopupInterface, workspace::Workspace, Footer, Tree};
use crate::events::messages::PopupMessage;
use std::{io::Stdout, path::PathBuf};

pub enum Mode {
    Select,
    Insert,
    Popup((Box<Mode>, Box<dyn PopupInterface>)),
}

impl From<&Mode> for Span<'static> {
    fn from(mode: &Mode) -> Self {
        match mode {
            Mode::Insert => Span::styled("  Insert  ", Style::default().fg(Color::Rgb(255, 0, 0))),
            Mode::Select => Span::styled("  Select  ", Style::default().fg(Color::LightCyan)),
            Mode::Popup((inner, _)) => match inner.as_ref() {
                Mode::Insert => Span::styled("  Insert  ", Style::default().fg(Color::Gray)),
                Mode::Select => Span::styled("  Select  ", Style::default().fg(Color::Gray)),
                Mode::Popup(..) => Span::styled("  Nested  ", Style::default().fg(Color::Gray)),
            },
        }
    }
}

impl Mode {
    pub fn render_popup_if_exists(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        if let Self::Popup((.., popup)) = self {
            popup.render(frame)
        }
    }

    pub fn popup(&mut self, popup: Box<dyn PopupInterface>) {
        match self {
            Self::Insert => self.popup_insert(popup),
            Self::Select => self.popup_select(popup),
            _ => {}
        }
    }

    pub fn popup_insert(&mut self, popup: Box<dyn PopupInterface>) {
        *self = Self::Popup((Box::new(Self::Insert), popup));
    }

    pub fn popup_select(&mut self, popup: Box<dyn PopupInterface>) {
        *self = Self::Popup((Box::new(Self::Select), popup));
    }

    pub fn clear_popup(&mut self) {
        if let Self::Popup((mode, _)) = self {
            match **mode {
                Self::Insert => *self = Self::Insert,
                _ => *self = Self::Select,
            }
        }
    }

    pub fn popup_map(&mut self, key: &KeyEvent) -> Option<PopupMessage> {
        if let Self::Popup((.., popup)) = self {
            return Some(popup.map(key));
        }
        None
    }

    pub fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Self::Popup((_, popup)) = self {
            popup.update_workspace(workspace);
        }
    }

    pub fn update_tree(&mut self, file_tree: &mut Tree) {
        if let Self::Popup((_, popup)) = self {
            popup.update_tree(file_tree);
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

impl From<&FileType> for &'static str {
    fn from(value: &FileType) -> Self {
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
    }
}

impl From<&FileType> for String {
    fn from(value: &FileType) -> String {
        let string: &'static str = value.into();
        string.to_owned()
    }
}
