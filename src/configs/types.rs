use serde::{Deserialize, Serialize};
use std::path::Path;

pub enum ScopeType {
    Marked { open: char, close: char },
    Indent,
    Text,
}

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum FileType {
    #[default]
    Text,
    MarkDown,
    Rust,
    Zig,
    C,
    Cpp,
    Nim,
    Python,
    JavaScript,
    TypeScript,
    Yml,
    Toml,
    Html,
    Lobster,
    Json,
    Shell,
}

impl FileType {
    pub fn derive_type(path: &Path) -> Self {
        let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
            return match path.file_name().and_then(|f| f.to_str()) {
                Some(".bashrc") => FileType::Shell,
                _ => Self::Text,
            };
        };
        match extension.to_lowercase().as_str() {
            "md" => Self::MarkDown,
            "rs" => Self::Rust,
            "zig" => Self::Zig,
            "c" => Self::C,
            "cpp" => Self::Cpp,
            "nim" => Self::Nim,
            "py" | "pyw" => Self::Python,
            "js" | "jsx" => Self::JavaScript,
            "ts" | "tsx" => Self::TypeScript,
            "yml" | "yaml" => Self::Yml,
            "toml" => Self::Toml,
            "html" => Self::Html,
            "lobster" => Self::Lobster,
            "json" => Self::Json,
            "sh" => Self::Shell,
            _ => Self::Text,
        }
    }

    pub fn comment_start(&self) -> &str {
        match self {
            Self::Python | Self::Toml | Self::Shell => "#",
            _ => "//",
        }
    }

    pub fn scope_type(&self) -> ScopeType {
        if !self.is_code() {
            return ScopeType::Text;
        }
        match self {
            Self::Python | Self::Nim | Self::Lobster => ScopeType::Indent,
            _ => ScopeType::Marked { open: '{', close: '}' },
        }
    }

    pub fn is_code(&self) -> bool {
        match self {
            Self::Text | Self::MarkDown => false,
            Self::Rust
            | Self::Zig
            | Self::C
            | Self::Cpp
            | Self::Nim
            | Self::Python
            | Self::JavaScript
            | Self::TypeScript
            | Self::Yml
            | Self::Toml
            | Self::Html
            | Self::Lobster
            | Self::Json
            | Self::Shell => true,
        }
    }

    pub const fn iter_langs() -> [Self; 14] {
        [
            Self::Rust,
            Self::Zig,
            Self::C,
            Self::Cpp,
            Self::Nim,
            Self::Python,
            Self::JavaScript,
            Self::TypeScript,
            Self::Yml,
            Self::Toml,
            Self::Html,
            Self::Lobster,
            Self::Json,
            Self::Shell,
        ]
    }

    pub const fn as_str(&self) -> &'static str {
        ft_to_str(*self)
    }
}

impl From<FileType> for &'static str {
    fn from(value: FileType) -> Self {
        ft_to_str(value)
    }
}

impl From<FileType> for String {
    fn from(value: FileType) -> String {
        ft_to_str(value).to_owned()
    }
}

pub const fn ft_to_str(value: FileType) -> &'static str {
    match value {
        FileType::Text | FileType::MarkDown => "unknown file type - error",
        FileType::Rust => "rust",
        FileType::Zig => "zig",
        FileType::C => "c",
        FileType::Cpp => "c++",
        FileType::Nim => "nim",
        FileType::Python => "python",
        FileType::JavaScript => "javascript",
        FileType::TypeScript => "typescript",
        FileType::Yml => "yaml",
        FileType::Toml => "toml",
        FileType::Html => "html",
        FileType::Lobster => "lobster",
        FileType::Json => "json",
        FileType::Shell => "shellscript",
    }
}
