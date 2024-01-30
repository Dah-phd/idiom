use std::path::PathBuf;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Html,
    C,
    Cpp,
    Yml,
    Toml,
    MarkDown,
    Unknown,
}

impl FileType {
    #[allow(clippy::ptr_arg)]
    pub fn derive_type(path: &PathBuf) -> Self {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            return match extension.to_lowercase().as_str() {
                "rs" => Self::Rust,
                "c" => Self::C,
                "cpp" => Self::Cpp,
                "py" | "pyw" => Self::Python,
                "md" => Self::MarkDown,
                "js" => Self::JavaScript,
                "ts" => Self::TypeScript,
                "yml" | "yaml" => Self::Yml,
                "toml" => Self::Toml,
                "html" => Self::Html,
                _ => Self::Unknown,
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
            FileType::Cpp => "c++",
            FileType::Yml => "yaml",
            FileType::Toml => "toml",
            FileType::MarkDown => "markdown",
            FileType::Unknown => "unknown",
        }
    }
}

impl From<&FileType> for String {
    fn from(value: &FileType) -> String {
        let string: &'static str = value.into();
        string.to_owned()
    }
}
