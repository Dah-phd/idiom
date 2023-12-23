use std::path::PathBuf;

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
