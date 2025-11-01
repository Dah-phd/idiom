use serde::{Deserialize, Serialize};
use std::path::Path;

pub enum ScopeType {
    Marked { opening: char, closing: char },
    Indent,
    Text,
}

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum FileFamily {
    #[default]
    Text,
    MarkDown,
    Code(FileType),
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
            return Self::Text;
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
        match self {
            Self::Python | Self::Nim | Self::Lobster => ScopeType::Indent,
            _ => ScopeType::Marked { opening: '{', closing: '}' },
        }
    }

    pub fn family(self) -> FileFamily {
        FileFamily::from(self)
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
}

impl From<FileFamily> for FileType {
    fn from(value: FileFamily) -> Self {
        match value {
            FileFamily::Text => FileType::Text,
            FileFamily::MarkDown => FileType::MarkDown,
            FileFamily::Code(file_type) => file_type,
        }
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

impl From<FileType> for FileFamily {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Text => FileFamily::Text,
            FileType::MarkDown => FileFamily::MarkDown,
            FileType::Rust
            | FileType::Zig
            | FileType::C
            | FileType::Cpp
            | FileType::Nim
            | FileType::Python
            | FileType::JavaScript
            | FileType::TypeScript
            | FileType::Yml
            | FileType::Toml
            | FileType::Html
            | FileType::Lobster
            | FileType::Json
            | FileType::Shell => FileFamily::Code(value),
        }
    }
}

const fn ft_to_str(value: FileType) -> &'static str {
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
