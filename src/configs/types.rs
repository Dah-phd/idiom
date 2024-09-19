use std::path::PathBuf;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy, Default)]
pub enum FileType {
    #[default]
    Ignored,
    Rust,
    Lobster,
    Zig,
    Python,
    JavaScript,
    TypeScript,
    Html,
    C,
    Cpp,
    Yml,
    Toml,
    Json,
    Nim,
}

impl FileType {
    #[allow(clippy::ptr_arg)]
    pub fn derive_type(path: &PathBuf) -> Option<Self> {
        let extension = path.extension().and_then(|e| e.to_str())?;
        match extension.to_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "zig" => Some(Self::Zig),
            "c" => Some(Self::C),
            "nim" => Some(Self::Nim),
            "cpp" => Some(Self::Cpp),
            "py" | "pyw" => Some(Self::Python),
            "js" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "yml" | "yaml" => Some(Self::Yml),
            "toml" => Some(Self::Toml),
            "html" => Some(Self::Html),
            "lobster" => Some(Self::Lobster),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub fn comment_start(&self) -> &str {
        match self {
            Self::Python => "#",
            _ => "//",
        }
    }
}

impl From<FileType> for &'static str {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Ignored => "unknown file type - error",
            FileType::Zig => "zig",
            FileType::Rust => "rust",
            FileType::Python => "python",
            FileType::TypeScript => "typescript",
            FileType::JavaScript => "javascript",
            FileType::Html => "html",
            FileType::Nim => "nim",
            FileType::C => "c",
            FileType::Cpp => "c++",
            FileType::Yml => "yaml",
            FileType::Toml => "toml",
            FileType::Lobster => "lobster",
            FileType::Json => "json",
        }
    }
}

impl From<FileType> for String {
    fn from(value: FileType) -> String {
        let string: &'static str = value.into();
        string.to_owned()
    }
}
